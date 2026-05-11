# Encoder に reconfigure を追加する

Created: 2026-05-10
Completed: 2026-05-10
Model: Opus 4.7

## 概要

エンコード中に動的にビットレートなどを変更する API がない。`Encoder::reconfigure(ReconfigureParams)` を追加し、libaom の `aom_codec_enc_config_set` を呼び出して設定を再適用する。

## 根拠

WebRTC / SFU 等の用途では、ネットワーク帯域に応じてエンコード中にターゲットビットレートを変更する必要がある。現状 `EncoderConfig` は `Encoder::new` でしか渡せず、再生成するとシーケンスが切れてしまうため動的変更には使えない。

姉妹プロジェクト nvcodec-rs では既に `Encoder::reconfigure(ReconfigureParams)` という形で同等機能を提供しているため、API 形状を揃える。

## 設計

### 公開 API

```rust
#[derive(Debug, Clone, Default)]
pub struct ReconfigureParams {
    /// ターゲットビットレート (kbps 単位, libaom: rc_target_bitrate)
    pub rc_target_bitrate: Option<u32>,
    // 必要に応じて他フィールドを追加していく
    // (max_intra_bitrate_pct など、aom_codec_enc_config_set / AV1E_SET_* で
    //  ランタイムに変更可能なものに限る)
}

impl Encoder {
    pub fn reconfigure(&mut self, params: ReconfigureParams) -> Result<(), Error>;
}
```

### 内部実装方針

1. 内部に保持している `aom_codec_enc_cfg_t` のコピーに対して、`Some` で指定されたフィールドのみ書き換える
2. `aom_codec_enc_config_set` を呼び出して反映する
3. 反映に成功したら、後続フレームの参照のため内部状態の `EncoderConfig` (もしくは raw cfg) も更新する

### 単位の取り扱い

- aom-rs の `rc_target_bitrate` は **kbps** (libaom 規約に従う、既存 `EncoderConfig::rc_target_bitrate` と同じ)
- nvcodec-rs は bps なので単位は揃わないが、各バックエンドの規約に従う方を優先する
- フィールドの doc コメントに単位を明記する

### 変更可能フィールドの範囲

`aom_codec_enc_config_set` で変更不可 / 未定義動作になるフィールド (`g_w`, `g_h`, `g_profile` 等) は `ReconfigureParams` に含めない。最初は `rc_target_bitrate` のみで切り、需要が出てから個別に追加する。

## 実装タスク

1. `ReconfigureParams` 構造体を `src/lib.rs` に追加する
2. `Encoder::reconfigure` を実装する (`aom_codec_enc_config_set` バインディング呼び出し)
3. PBT / unittest を追加する
   - 初期ビットレートとは異なる値を `reconfigure` で適用したあとに数フレームエンコードして、エラーが出ないこと
   - `Encoder::new` 直後の `reconfigure` も成功すること
4. `CHANGES.md` の `## develop` に `[ADD]` で追記する
5. nvcodec-rs との API 比較を README なり doc コメントなりで揃える

## 参考

- nvcodec-rs `ReconfigureParams` / `Encoder::reconfigure` (src/encode.rs:399, src/encode.rs:652)
- libaom `aom_codec_enc_config_set`

## 解決方法

`src/lib.rs` に以下を追加した。

- `ReconfigureParams` 構造体: `g_w`, `g_h`, `g_timebase`, `rc_target_bitrate` を `Option` で持つ
  - 設計セクションでは `rc_target_bitrate` のみ公開する方針だったが、ランタイムでの解像度・タイムベース変更も含めて libaom 側 API に揃えるため初期実装では追加していた。これらは後に 0006 で削除されている (`g_w` / `g_h` は内部 `plane_sizes` / `aom_image` バッファが追従しない、`g_timebase` は libwebrtc に倣い固定運用とする方針のため)
- `Encoder` 構造体に `cfg: sys::aom_codec_enc_cfg` フィールドを追加し、初期化時の cfg を保持する
- `Encoder::reconfigure(&mut self, params: ReconfigureParams)`: `Some` のフィールドのみを self.cfg に書き戻し、`aom_codec_enc_config_set()` で適用する
  - エンコード結果取り出し中 (`iter` が非 NULL) は呼べない (`encode` / `finish` と同じガード)

`max_bitrate` (nvcodec-rs にはあるフィールド) は libaom に対応する単一フィールドが存在しないため公開していない。必要になった時点で `rc_2pass_vbr_maxsection_pct` 等を別途検討する。

`tests/test_roundtrip.rs` に以下のテストを追加した。

- `test_reconfigure_target_bitrate_midstream`: エンコード途中でビットレートを変更しても完走する
- `test_reconfigure_immediately_after_new`: `Encoder::new` 直後の reconfigure が成功する
- `test_reconfigure_empty_params_is_noop`: 空の `ReconfigureParams` で no-op として動作する

## 振り返り

本 issue は機能追加そのものとしては必要だったが、**初期実装が設計通りに完成しなかったため、派生 issue を多数発生させる起点になった**。

派生・連鎖した issue:

- **0005** (FFI 失敗時の状態ロールバック): 0004 で `self.cfg` を直接書き換えてから FFI を呼ぶ初歩設計欠陥を作り込んだため発生。0004 で「複製 → 変更 → 成功時確定」パターンを最初から採用していれば不要。
- **0006** (`g_w` / `g_h` / `g_timebase` 削除): 設計セクションで「`rc_target_bitrate` のみで切る」と決めていたにも関わらず実装段階でスコープを広げたため発生。0004 で設計通り 1 フィールドだけにしていれば不要。
- **0007** (`g_timebase.den == 0` バリデーション): 0006 で前提消滅。0004 が `g_timebase` を含めていなければそもそも検討されなかった。
- **0009** (テストカバレッジ拡充): 0004 の初期テスト 3 本が成功パスのみ・効果検証なしだったため拡充が必要になった。0004 段階で iter active / VBR / finish 後 / state ロールバック / ビットレート効果 まで網羅していれば不要。
- **0010** (`Default` derive 撤廃): 姉妹クレート (`amf-rs` / `vpl-rs` / `nvcodec-rs`) 確認なしに撤廃 → 最終的に復活、完全に空振り。
- **0011** (コードクリーンアップ): `iter.is_null()` 3 重複、自明コメント、CHANGES.md 文言整え、doc 重複の修正。0004 段階でコードレビュー観点を押さえていれば不要。
- **0012** (libwebrtc doc + example): example は残ったが doc 追加部分は後段レビューで全面撤回。0004 段階で「API doc は契約だけ、内部事情は別 doc」と切り分けていれば doc 部分は最初から書かれなかった。

教訓:

1. **設計セクションのスコープを実装で広げない**。広げる場合は別 issue で根拠を明示してから着手する。
2. **FFI 失敗時のロールバック / unsafe の選択 / bindgen 生成型の derive 確認** を最初に押さえる。
3. **テスト戦略** (成功パス・エラーパス・効果検証・境界値・定数化・エラー比較方式) を機能追加と同じ issue 内で完結させる。テスト拡充 issue を切る運用は書き漏らしのサイン。
4. **姉妹クレート (amf-rs / vpl-rs / nvcodec-rs) の同名型を最初に確認する**。設定構造体・エラー型・列挙型で derive やシグネチャを揃える判断は機能追加の段階で済ませる。
5. **API リファレンスにはライブラリ内部事情 (libwebrtc に倣う / 将来の SVC 拡張) を書かない**。`issues/` のパスは配布物に残らないので doc から参照しない。
6. **コードレビュー観点** (コピペ重複 / 自明コメント / doc 重複 / `as _` の型明示 / Display 用エラー比較) を機能追加の段階で消化する。
