# 変更履歴

- CHANGE
  - 後方互換のない変更
- ADD
  - 後方互換がある追加
- UPDATE
  - 後方互換がある変更
- FIX
  - バグ修正

## develop

## 2026.2.0

**リリース日**: 2026-08-12

- [CHANGE] MSRV (`rust-version`) を 1.88 から 1.93 に引き上げる
  - @voluntas
- [CHANGE] `KeyframeMode` に `Disabled` variant を追加する
  - @voluntas
- [CHANGE] `EncodingPass` を OnePass のみに削減し、マルチパスエンコードを非対応とする
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
- [ADD] `pbt/` に proptest による Property-Based Testing を追加する
  - @voluntas
- [ADD] `fuzz/` に cargo-fuzz によるデコーダー fuzz ターゲットを追加する
  - @voluntas
- [ADD] ubuntu-26.04 / ubuntu-26.04-arm の prebuilt バイナリをサポートする
  - @voluntas
- [UPDATE] libaom を v3.13.2 から v3.14.1 に更新する
  - @voluntas
- [FIX] `KeyframeMode::Fixed` の rustdoc を libaom 実態 (`AOM_KF_DISABLED` の deprecated エイリアス) に合わせて修正し、`#[deprecated]` 属性を付与する
  - @voluntas
- [FIX] `DOCS_RS=1` ビルドでダミー bindings に `aom_kf_mode` 型と関連定数が不足し `cargo doc --no-deps` が失敗する問題を修正する
  - @voluntas
- [FIX] `Encoder::init` の unsafe コードに `DecodedFrame::plane` と同等の防御的検証（planes null チェック・stride 正値チェック・`checked_mul` + `isize::MAX` オーバーフローチェック）を追加する
  - @voluntas
- [FIX] 16-bit フォーマット (I42016 / I42216 / I44416) でエンコードする際に `AOM_CODEC_USE_HIGHBITDEPTH` フラグを init 時に付与し、エンコードが失敗する問題を修正する
  - @voluntas
- [FIX] `Encoder::new` で画像フォーマット × `g_profile` × ビット深度の整合を事前検証し、「init 成功 → 初回 `encode` での遅延失敗」を解消する
  - @voluntas
- [FIX] DOCS_RS ビルドのダミー bindings が src/lib.rs が参照するシンボルを欠落し `DOCS_RS=1 cargo build` が失敗する問題を、全シンボルをカバーする形に刷新して修正する
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
- [ADD] `Encoder` / `Decoder` のスレッド間移動検証テストを追加する
  - @voluntas
- [UPDATE] build-dependencies の `shiguredo_cmake` を 4.3 から 4.4 に更新する
  - @voluntas
- [UPDATE] `pbt/` の `proptest` を 1.7 から 1.11 に更新する
  - @voluntas
- [UPDATE] `Encoder::encode()` / `finish()` / `reconfigure()` の `next_frame()` ガードを `check_iter_drained` ヘルパーに集約する
  - @voluntas
- [UPDATE] `KeyframeMode` から libaom の `aom_kf_mode` 定数へのマッピングを `map_kf_mode` private fn に切り出す
  - @voluntas
- [UPDATE] lint 抑制を `#[allow]` から `#[expect]` に統一する
  - @voluntas
- [UPDATE] Copy 可能な `PlaneSizes` に `Copy` を derive する
  - @voluntas
- [UPDATE] `prek.toml` を shiguredo-rust 規約に合わせて更新する (`cargo test` を pre-push のみ、tombi 追加)
  - @voluntas
- [UPDATE] 統合テストを `tests/roundtrip.rs` と `tests/helpers/` に再配置する
  - @voluntas
- [UPDATE] CI / release ワークフローの composite action 参照を SHA ピン留めにする
  - @voluntas
- [UPDATE] CI / release ワークフローの Slack 通知とリリース手順を修正する (slack_notify の `always()` / `GH_TOKEN`、タグと Cargo.toml のバージョン照合、正式リリースの draft 化)
  - @voluntas
- [FIX] `canary.py` のバージョン検出・変換を Cargo.toml の [package] セクションの version 行に限定し、rust-version 等への誤マッチを防ぐ
  - @voluntas

## 2026.1.0

**リリース日**: 2026-03-31
