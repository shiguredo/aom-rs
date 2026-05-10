# ReconfigureParams から g_w / g_h / g_timebase を削除する

Created: 2026-05-10
Completed: 2026-05-10
Model: deepseek v4-pro

## 概要

`ReconfigureParams` に `g_w` / `g_h` / `g_timebase` フィールドが定義されているが、いずれもランタイム再設定としては正しく機能しない、もしくは libwebrtc の運用パターンと整合しないため削除する。

- `g_w` / `g_h`: `reconfigure()` で変更しても内部状態 (`self.plane_sizes`, `self.img`) が更新されず、初期値と異なる解像度での動作が保証できない
- `g_timebase`: libwebrtc の AV1 エンコーダー実装は timebase を初期化時に `{1, 90000}` で固定し、ランタイムでは動かさない。aom-rs もこの運用 (`0012-enh-add-libwebrtc-style-reconfigure`) を優先するため、ReconfigureParams から外す

本変更は公開 API フィールドの削除であり、後方互換のない変更（`[CHANGE]`）である。ただし `ReconfigureParams` 自体が未リリース（`## develop` セクション内）のため、実利用者への影響はない。

## 根拠

### 1. plane_sizes が初期化時のまま

`self.plane_sizes` は `Encoder::init()` (`src/lib.rs:1534-1576`) で初期 `g_w` / `g_h` から計算され、`reconfigure()` では再計算されない。

`encode()` 内のプレーンサイズ検証 (`src/lib.rs:2044-2084`) が `self.plane_sizes` を参照するため、`g_w` / `g_h` を初期値と異なる値に変更すると新しい解像度の画像データが reject される。

### 2. self.img のバッファが初期サイズのまま

`self.img` は `init()` 内で `aom_img_alloc()` (`src/lib.rs:1517-1531`) によって初期解像度に合わせて確保される。`reconfigure()` で解像度を拡大した場合、`encode()` 内の `copy_from_slice` (`src/lib.rs:2102-2109`) でバッファオーバーフローの危険がある。

### 3. 設計との不一致

Issue `0004-feature-add-encoder-reconfigure` の設計セクションに「`aom_codec_enc_config_set` で変更不可 / 未定義動作になるフィールド (`g_w`, `g_h`, `g_profile` 等) は `ReconfigureParams` に含めない。最初は `rc_target_bitrate` のみで切り、需要が出てから個別に追加する。」と明記されている。

### 4. self.cfg.g_w / self.cfg.g_h は他で参照されていない

`self.cfg.g_w` / `self.cfg.g_h` は `reconfigure()` 内 (`src/lib.rs:2003, 2006`) での代入以外に、読み取りすらされていない。削除しても既存コードの動作に一切影響しない。

### 5. g_timebase は libwebrtc 方式に倣い固定運用とする

libwebrtc の AV1 エンコーダー実装 (`webrtc/src/modules/video_coding/codecs/av1/libaom_av1_encoder.cc`) では `g_timebase` を初期化時に `{1, kVideoPayloadTypeFrequency}` (= `{1, 90000}`, RTP の 90kHz) で固定し、`SetRates()` でも触らない。フレームレート変動はエンコード時の duration（PTS 差分）で表現される。

aom-rs もこの方針 (`0012-enh-add-libwebrtc-style-reconfigure`) に揃えるため、`g_timebase` をランタイム変更可能フィールドから外す。`reconfigure()` で den == 0 を弾くバリデーション (旧 0007) もフィールド消滅により不要となる。

## 修正方針

### 対象ファイル: `src/lib.rs`

**1. ReconfigureParams 構造体定義から g_w / g_h / g_timebase フィールドを削除する** (`src/lib.rs:1258-1269`)

```rust
// 削除対象
pub g_w: Option<u32>,
pub g_h: Option<u32>,
pub g_timebase: Option<AomRational>,
```

各フィールドの doc コメントも同時に削除する。`AomRational` の import は他で使われていれば残す。

**2. ReconfigureParams 構造体 doc を残存フィールドに適した内容に書き直す** (`src/lib.rs:1251-1255`)

g_w/g_h/g_timebase 削除後、残存フィールドは `rc_target_bitrate` のみとなる。L1251-1255 の文面は g_w/g_h を前提としているため、書き直しが必要。

```rust
// 変更前
/// libaom の `aom_codec_enc_config_set()` で変更可能なフィールドのうち、
/// 帯域適応用途で典型的に必要となるものを公開している。
/// 解像度を変更する場合は、エンコーダー初期化時に
/// [`EncoderConfig::g_forced_max_frame_width`] /
/// [`EncoderConfig::g_forced_max_frame_height`] を設定しておく必要がある。

// 変更後
/// libaom の `aom_codec_enc_config_set()` で変更可能なフィールドのうち、
/// ランタイム変更が安全なものを公開している。libwebrtc の AV1 エンコーダー実装に倣い、
/// timebase は初期化時に固定し、ランタイムでは動かさない方針を採る。
```

**3. reconfigure() メソッドから g_w / g_h / g_timebase 代入ブロックを削除する** (`src/lib.rs:2002-2011`)

```rust
// 削除対象
if let Some(v) = params.g_w {
    self.cfg.g_w = v as _;
}
if let Some(v) = params.g_h {
    self.cfg.g_h = v as _;
}
if let Some(v) = params.g_timebase {
    self.cfg.g_timebase.num = v.num as c_int;
    self.cfg.g_timebase.den = v.den as c_int;
}
```

**4. reconfigure() メソッド doc から g_w/g_h 関連記述を削除する** (`src/lib.rs:1989-1992`)

```rust
// 削除対象 (この 4 行全体)
/// 解像度 (`g_w` / `g_h`) を変更する場合は、エンコーダー初期化時に
/// [`EncoderConfig::g_forced_max_frame_width`] /
/// [`EncoderConfig::g_forced_max_frame_height`] を指定しておく必要がある。
/// 値がそれを超えるか libaom の制約に違反すると、エラーが返される。
```

### 対象ファイル: `tests/test_roundtrip.rs`

既存テストで `g_timebase: Some(...)` を使っている箇所があれば、削除する。`test_reconfigure_immediately_after_new` 等で `g_timebase` を渡している場合は該当行を消す。

### 対象ファイル: `CHANGES.md`

`## develop` の `[ADD]` エントリより後（`[CHANGE]` の位置）に以下を追記する:

```
- [CHANGE] `ReconfigureParams` から `g_w` / `g_h` / `g_timebase` フィールドを削除する
  - @voluntas
```

### 確認手順

1. `cargo build` が成功すること
2. `cargo test` で全テストが通過すること。既存の reconfigure テスト (`test_reconfigure_target_bitrate_midstream`, `test_reconfigure_immediately_after_new`, `test_reconfigure_empty_params_is_noop`) は `rc_target_bitrate` のみを使う前提でテストを更新する
3. `cargo clippy --all-targets --all-features -- -D warnings` が通過すること

## 他 issue への影響

- **0005** (`bug-fix-reconfigure-state-inconsistency`) は本 issue (0006) の後に適用することを推奨。0006 が先に適用されれば、0005 の修正コードから `g_w` / `g_h` / `g_timebase` 代入部分が不要になる
- **0007** (`bug-validate-reconfigure-den-zero`) は本 issue のスコープ拡張により不要となり close 済み（`issues/closed/0007-...`）
- **0010** (`enh-remove-reconfigure-default-derive`) も `ReconfigureParams` 構造体定義を操作するため、本 issue (0006) を先に適用する必要がある
- **0011** (`misc-cleanup-reconfigure-code`) の doc 重複削除は本 issue の修正内容と重複する可能性がある。0006 を先に適用し、0011 は 0006 の適用後内容に対して適用する
- **0012** (`enh-add-libwebrtc-style-reconfigure`) は本 issue で `g_timebase` 削除が完了した後の doc / example 整備を担う。0006 → 0012 の順で適用する
- **0013** (`enh-add-svc-runtime-control`, `issues/pending/`) は SVC 統合の設計判断が必要なため pending。本 issue とは独立

## 解決方法

`src/lib.rs` の `ReconfigureParams` から `g_w` / `g_h` / `g_timebase` フィールドと関連 doc を削除し、`Encoder::reconfigure()` 内の対応する代入ブロックも削除した。`reconfigure()` の doc コメントから解像度変更前提の注意書きも除去した。

`tests/test_roundtrip.rs` から `AomRational` import および `g_timebase` を渡している記述を削除し、`..Default::default()` が無意味になった箇所も整理した。

`CHANGES.md` の `## develop` に `[CHANGE]` エントリを追記した。

`cargo test`、`cargo clippy --all-targets --all-features -- -D warnings` がいずれも通過することを確認した。
