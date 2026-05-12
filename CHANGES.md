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
- [CHANGE] `KeyframeMode` に `#[non_exhaustive]` を付与する
  - @voluntas
- [CHANGE] `KeyframeMode` に `Disabled` variant を追加する
  - @voluntas
- [FIX] `KeyframeMode::Fixed` の rustdoc を libaom 実態 (`AOM_KF_DISABLED` の deprecated エイリアス) に合わせて修正し、`#[deprecated]` 属性を付与する
  - @voluntas

### misc

- `examples/midstream_reconfigure.rs` を追加し、30fps エンコード途中でビットレートを切り替える典型パターンを示す
  - @voluntas
- `Encoder::encode()` / `finish()` / `reconfigure()` の `next_frame()` ガードを `check_iter_drained` ヘルパーに集約する
  - @voluntas
- reconfigure 周辺の単体テストを拡充する (ビットレート反映の検証 / PSNR 検証 / force_keyframe との併用 / 失敗時のロールバック検証)
  - @voluntas

## 2026.1.0

**リリース日**: 2026-03-31
