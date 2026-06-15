# 変更履歴

- UPDATE
  - 後方互換がある変更
- ADD
  - 後方互換がある追加
- CHANGE
  - 後方互換のない変更
- FIX
  - バグ修正

## develop

- [CHANGE] `EncoderConfig` に `#[non_exhaustive]` を付与する
  - @voluntas
- [CHANGE] `KeyframeMode` に `#[non_exhaustive]` を付与する
  - @voluntas
- [CHANGE] `KeyframeMode` に `Disabled` variant を追加する
  - @voluntas
- [ADD] `Encoder::reconfigure(&ReconfigureParams)` を追加してターゲットビットレートをランタイム変更可能にする
  - @voluntas
- [ADD] `EncoderConfig` に `enable_order_hint` フィールドを追加する
  - @voluntas
- [ADD] `EncoderConfig` に `enable_ref_frame_mvs` フィールドを追加する
  - @voluntas
- [ADD] `EncoderConfig` に `enable_angle_delta` フィールドを追加する
  - @voluntas
- [ADD] `EncoderConfig` に `intra_default_tx_only` フィールドを追加する
  - @voluntas
- [ADD] `EncoderConfig` に `coeff_cost_upd_freq` フィールドを追加する
  - @voluntas
- [ADD] `EncoderConfig` に `mode_cost_upd_freq` フィールドを追加する
  - @voluntas
- [ADD] `EncoderConfig` に `mv_cost_upd_freq` フィールドを追加する
  - @voluntas
- [UPDATE] libaom を v3.13.2 から v3.14.1 に更新する
  - @voluntas
- [FIX] `KeyframeMode::Fixed` の rustdoc を libaom 実態 (`AOM_KF_DISABLED` の deprecated エイリアス) に合わせて修正し、`#[deprecated]` 属性を付与する
  - @voluntas
- [FIX] `DOCS_RS=1` ビルドでダミー bindings に `aom_kf_mode` 型と関連定数が不足し `cargo doc --no-deps` が失敗する問題を修正する
  - @voluntas

### misc

- [ADD] `examples/midstream_reconfigure.rs` を追加し、30fps エンコード途中でビットレートを切り替える典型パターンを示す
  - @voluntas
- [ADD] reconfigure 周辺の単体テストを拡充する (ビットレート反映の検証 / PSNR 検証 / force_keyframe との併用 / 失敗時のロールバック検証)
  - @voluntas
- [ADD] realtime 制御フラグ周辺のテストを拡充する (`*_cost_upd_freq` 値域エラー / `KeyframeMode::Auto` / `KeyframeMode::Fixed` の deprecated alias 同値性)
  - @voluntas
- [ADD] CI に ubuntu-26.04 / ubuntu-26.04-arm を追加する
  - @voluntas
- [UPDATE] `Encoder::encode()` / `finish()` / `reconfigure()` の `next_frame()` ガードを `check_iter_drained` ヘルパーに集約する
  - @voluntas
- [UPDATE] `KeyframeMode` から libaom の `aom_kf_mode` 定数へのマッピングを `map_kf_mode` private fn に切り出す
  - @voluntas
- [UPDATE] `README.md` の `EncoderConfig` / `KeyframeMode` セクションに realtime 配信向けの追加フィールドと `#[non_exhaustive]` 注記を反映する
  - @voluntas

## 2026.1.0

**リリース日**: 2026-03-31
