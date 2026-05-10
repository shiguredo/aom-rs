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
