# realtime 配信向けの制御フラグを EncoderConfig と KeyframeMode に追加する

Created: 2026-05-12
Model: Opus 4.7

## 概要

AV1 のリアルタイム配信 (WebRTC / SFU 用途) で典型的に必要となる sequence ヘッダーフラグ・エンコーダー速度トレードオフ・外部キーフレーム制御を、aom-rs の `EncoderConfig` から指定できるようにする。

`EncoderConfig` に 7 フィールド、`KeyframeMode` に 1 variant を追加する (計 8 項目)。いずれも sequence-level あるいは初期化時に 1 回設定すれば足りるフラグであり、midstream での更新は不要 (= `ReconfigureParams` への追加は本 issue では行わない)。

## 追加対象

| 追加するフィールド / variant | 対応する libaom 定数 | 推奨値 | 説明 |
| --- | --- | --- | --- |
| `enable_order_hint: Option<bool>` | `AV1E_SET_ENABLE_ORDER_HINT` | `false` | frame order hint を無効化。joint compound mode / motion field MV / ref frame sign bias を停止する |
| `enable_ref_frame_mvs: Option<bool>` | `AV1E_SET_ENABLE_REF_FRAME_MVS` | `false` | ref frame mvs (mfmv) を無効化 |
| `enable_angle_delta: Option<bool>` | `AV1E_SET_ENABLE_ANGLE_DELTA` | `false` | intra 角度予測の delta を無効化 |
| `intra_default_tx_only: Option<bool>` | `AV1E_SET_INTRA_DEFAULT_TX_ONLY` | `true` | intra mode で default TX type のみ使う (TX search 削減) |
| `coeff_cost_upd_freq: Option<u32>` | `AV1E_SET_COEFF_COST_UPD_FREQ` | `3` | 係数コスト更新を OFF にする (有効値 `0..=3`, `0=SB`, `1=SB row`, `2=tile`, `3=off`) |
| `mode_cost_upd_freq: Option<u32>` | `AV1E_SET_MODE_COST_UPD_FREQ` | `3` | モードコスト更新を OFF にする (有効値同上) |
| `mv_cost_upd_freq: Option<u32>` | `AV1E_SET_MV_COST_UPD_FREQ` | `3` | MV コスト更新を OFF にする (有効値同上) |
| `KeyframeMode::Disabled` (variant 追加) | `AOM_KF_DISABLED` (= 0) | (該当なし) | エンコーダー側のキーフレーム自動配置を停止し、外部から `EncodeOptions::force_keyframe = true` で挿入する運用に合わせる |

## 根拠

### sequence-level / フレームレベルの bitstream 機能制御

- `AV1E_SET_ENABLE_ORDER_HINT`
- `AV1E_SET_ENABLE_REF_FRAME_MVS`
- `AV1E_SET_ENABLE_ANGLE_DELTA`

いずれも AV1 のビットストリーム機能フラグで、低遅延配信では機能を絞ることで RD 探索コストとビットストリーム複雑度を抑えるのが一般的。midstream で切り替えるとビットストリーム互換 (シーケンスヘッダー / フレームヘッダー) が壊れるため、`EncoderConfig` (= `Encoder::new` 時) で 1 回だけ指定する API で十分。

`enable_ref_frame_mvs` はシーケンスヘッダーの `enable_ref_frame_mvs` フィールドが libaom 内部で `extra_cfg->enable_ref_frame_mvs & extra_cfg->enable_order_hint` の積として書き込まれるため、`enable_order_hint` を `false` に指定すると mfmv はシーケンスレベルで実質的に無効化される。利用者は通常両方を OFF にする組み合わせで指定する (rustdoc にこの依存関係を明記する)。

### エンコーダー速度トレードオフ

- `AV1E_SET_COEFF_COST_UPD_FREQ`
- `AV1E_SET_MODE_COST_UPD_FREQ`
- `AV1E_SET_MV_COST_UPD_FREQ`
- `AV1E_SET_INTRA_DEFAULT_TX_ONLY`

libaom 内部の探索コスト更新頻度・intra TX search 削減を制御する control 群。realtime プロファイルでは cost 更新を OFF (値 3) にして per-frame の RD コスト再計算を省くのが定型。これらはいずれも init 時に 1 度だけセットし、midstream では更新しない。

### キーフレーム制御と `KeyframeMode` の現状

外部から `EncodeOptions::force_keyframe = true` のみでキーフレームを駆動し、エンコーダー側の自動挿入・シーンチェンジ検出を完全に止める運用を素直に表現するため、`KeyframeMode` に `Disabled` variant を追加する。

libaom の `aom_encoder.h` 定義では `AOM_KF_FIXED` が `AOM_KF_DISABLED` の deprecated エイリアスとなっており (両者とも整数値 0)、`AOM_KF_AUTO = 1`。これに対し現状の `KeyframeMode::Fixed` の rustdoc コメント (`src/lib.rs:599` 付近の `/// 固定間隔`) は libaom 仕様と矛盾している (`Fixed` は libaom 上では「固定間隔」ではなく「自動配置の停止」を意味する deprecated 名)。本 issue で同時に修正する。

`KeyframeMode::Fixed` は `#[deprecated]` 属性を付与し、新規利用者には `KeyframeMode::Disabled` への移行を促す形にする。kf_mode マッピングは両者とも整数値 0 (DISABLED) で同一であるため、libaom 上の挙動は変わらない。`Fixed` の削除タイミングは本 issue では決定しない (削除する場合は別 issue で扱う)。

`AOM_KF_DISABLED` 指定時は libaom 内部で `auto_key = false` となり、`kf_min_dist` / `kf_max_dist` に基づく自動キーフレーム挿入は停止する (libaom `av1/av1_cx_iface.c` の `set_encoder_config` で確認)。ただし `kf_min_dist` / `kf_max_dist` 自体は内部 `key_freq_min` / `key_freq_max` にコピーされ続け、`kf_max_dist == 0` を別途指定した場合は `enable_keyframe_filtering` / `mv_cost_upd_freq` 等の派生設定が libaom 側で上書きされる副作用がある。本 issue で追加するテストでは `kf_min_dist` / `kf_max_dist` をデフォルト (`None`) のまま使用する。

### 後方互換とブランチ命名

`KeyframeMode` は現状 `#[non_exhaustive]` が付与されていないため、variant 追加は外部利用者の網羅 `match` を破壊する後方互換のない変更となる。本 issue では同時に `#[non_exhaustive]` を付与し、将来の variant 追加も含めて後方互換性を確保する。`#[non_exhaustive]` 付与自体も既存の網羅 `match` を破壊する変更であり、本 issue の `KeyframeMode` 関連変更は `[CHANGE]` カテゴリで扱う。

## 設計

### 1. `EncoderConfig` への 7 フィールド追加

`src/lib.rs` の `EncoderConfig` 末尾、`max_reference_frames` の後に以下のフィールドを追加する。並び順・doc コメントの体裁は既存フィールド (`enable_keyframe_filtering` 等) と揃える。

```rust
/// AV1E_SET_ENABLE_ORDER_HINT: frame order hint 有効化
pub enable_order_hint: Option<bool>,

/// AV1E_SET_ENABLE_REF_FRAME_MVS: ref frame mvs (mfmv) 有効化
///
/// `enable_order_hint` を `false` に指定した場合、シーケンスヘッダーの `enable_ref_frame_mvs`
/// は libaom 内部で 0 として書き込まれるため、mfmv は実質的に無効化される。
pub enable_ref_frame_mvs: Option<bool>,

/// AV1E_SET_ENABLE_ANGLE_DELTA: intra 角度予測 delta 有効化
pub enable_angle_delta: Option<bool>,

/// AV1E_SET_INTRA_DEFAULT_TX_ONLY: intra で default TX type のみ使う (TX search 削減)
pub intra_default_tx_only: Option<bool>,

/// AV1E_SET_COEFF_COST_UPD_FREQ: 係数コスト更新頻度 (0: SB, 1: SB row, 2: tile, 3: off)
pub coeff_cost_upd_freq: Option<u32>,

/// AV1E_SET_MODE_COST_UPD_FREQ: モードコスト更新頻度 (同上)
pub mode_cost_upd_freq: Option<u32>,

/// AV1E_SET_MV_COST_UPD_FREQ: MV コスト更新頻度 (同上)
pub mv_cost_upd_freq: Option<u32>,
```

`EncoderConfig::new()` の初期化ではすべて `None` で初期化する (= libaom のデフォルト値を尊重)。

`Encoder::apply_controls()` の末尾 (`AV1E_SET_MAX_REFERENCE_FRAMES` 呼び出しの後) に 7 件の `AV1E_SET_*` 呼び出しを追加する。`Some` の場合のみ対応する `AV1E_SET_*` を `set_control()` 経由で呼ぶ。bool は既存 `enable_*` 系と同じく `if v { 1 } else { 0 }`、u32 は `v as c_int` で渡す (既存 `enable_keyframe_filtering` / `aq_mode` 等と同一パターン)。

値域チェックは aom-rs 側でガードしない。`*_cost_upd_freq` で `0..=3` 以外を渡した場合は libaom の `aom_codec_control` がエラーを返し、`Encoder::new` がそのエラーを伝播する。これは既存の `aq_mode` (0..=3) 等と同じ流儀。

bindings.rs 上 `*_cost_upd_freq` の C 側 control type は `c_uint` だが、既存 `set_control` のシグネチャは `c_int` で、`enable_keyframe_filtering` などの他の `c_uint` パラメータも `c_int` 経由で呼んでいる。本 issue では既存流儀に揃え、型不整合の解消はスコープ外とする。

### 2. `KeyframeMode` の変更

`KeyframeMode` enum に以下の 3 つの変更を加える。

1. `#[non_exhaustive]` 属性を付与する
2. `Disabled` variant を追加する
3. 既存 `Fixed` の rustdoc を libaom 実態に合わせて修正し、`#[deprecated]` 属性を付与する

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeyframeMode {
    /// エンコーダー側のキーフレーム自動配置を停止する
    ///
    /// 外部から `EncodeOptions::force_keyframe = true` を指定したフレームのみが
    /// キーフレームになる。
    Disabled,

    /// `AOM_KF_FIXED` (libaom の deprecated エイリアス、`Disabled` と同一挙動)
    #[deprecated(note = "AOM_KF_FIXED is a deprecated alias of AOM_KF_DISABLED in libaom; use KeyframeMode::Disabled")]
    Fixed,

    /// エンコーダーが最適配置を自動決定する
    Auto,
}
```

`#[deprecated(note = ...)]` の `note` 文字列はコンパイラ警告メッセージとして表示されるため、CLAUDE.md のルール上は英語で記述する (rustdoc 本文は日本語のまま)。移行案内は `note` 側に集約し、rustdoc 本文には書かない。

`Encoder::init()` の kf_mode マッピング (`src/lib.rs:1414-1419`) を以下のように拡張する。`Fixed` variant が deprecated になるため `match` 式に `#[allow(deprecated)]` を直接付与する。あわせて `Fixed` と `Disabled` が同一定数値 (DISABLED, 整数値 0) にマップされる事実から `clippy::match_same_arms` が発火する可能性があるため、`#[allow(clippy::match_same_arms)]` も同時に付与する。

```rust
if let Some(mode) = config.kf_mode {
    aom_config.kf_mode = #[allow(deprecated, clippy::match_same_arms)]
    match mode {
        KeyframeMode::Disabled => sys::aom_kf_mode_AOM_KF_DISABLED,
        KeyframeMode::Fixed => sys::aom_kf_mode_AOM_KF_FIXED,
        KeyframeMode::Auto => sys::aom_kf_mode_AOM_KF_AUTO,
    };
}
```

`#[allow(...)]` を match 式の直前に置く構文は安定 Rust の expression attribute としてサポートされているが、stable のバージョン差異を踏むケースがあるため、実装着手時に `cargo check` で警告が抑止されることを確認すること。期待どおりに抑止されない場合は match を返す private fn (`fn map_kf_mode(mode: KeyframeMode) -> sys::aom_kf_mode`) に切り出し、その関数アイテムに `#[allow(deprecated, clippy::match_same_arms)]` を付与する代替案を採る。

`AOM_KF_FIXED` は libaom が deprecated alias として `AOM_KF_DISABLED` と同値であることを保証しているため、上記 2 つの arm が将来分岐する可能性は事実上ない。`#[allow(clippy::match_same_arms)]` は永続的に許容する。

実装着手時に `KeyframeMode::Fixed` の参照箇所を `rg "KeyframeMode::Fixed"` で確認すること (`src/lib.rs` の match arm 1 箇所のみであれば追加対応は不要、テストや examples で参照があれば移行を行うか `#[allow(deprecated)]` を付与する)。

### 3. テスト

`tests/test_roundtrip.rs` に既存ヘルパー (`realtime_config`, `roundtrip_colorbar`, `EncodedFrame::is_keyframe`) を流用して以下のテストを追加する (新規ファイルは作らない)。

テスト 2 / 3 は出力フレーム並びの決定論性が必要なため、`config.g_lag_in_frames = Some(0)` を明示的に上書きする (現状の `realtime_config` は `g_lag_in_frames` をセットしないため、aom-rs テストの自衛として明示する)。

1. **realtime 典型セットでの roundtrip / PSNR 検証** (関数名例: `test_roundtrip_realtime_controls_typical_set`)
   - `realtime_config(320, 240, RateControlMode::Cbr)` をベースに、追加 7 フィールドを推奨値 (`enable_order_hint = Some(false)`, `enable_ref_frame_mvs = Some(false)`, `enable_angle_delta = Some(false)`, `intra_default_tx_only = Some(true)`, `coeff_cost_upd_freq = Some(3)`, `mode_cost_upd_freq = Some(3)`, `mv_cost_upd_freq = Some(3)`) で埋め、さらに `kf_mode = Some(KeyframeMode::Disabled)` を指定
   - `roundtrip_colorbar(config, 30, 25.0)` を呼び、encode → decode 完走と PSNR 25dB 以上を確認 (既存 320x240 系テストと同じ閾値)
2. **`KeyframeMode::Disabled` 下での自動キーフレーム停止確認** (関数名例: `test_keyframe_mode_disabled_suppresses_auto_keyframe`)
   - テスト 1 と同じく `realtime_config(320, 240, RateControlMode::Cbr)` から再構築して同じ 7 フィールドと `kf_mode = Some(KeyframeMode::Disabled)` を設定し、さらに `config.g_lag_in_frames = Some(0)` を明示
   - 30 フレームを `EncodeOptions { force_keyframe: false }` で encode
   - `EncodedFrame::is_keyframe()` で各フレームを判定し、先頭 1 枚のみ `true`、それ以外 29 枚は `false` であることを厳密に assert する (`g_lag_in_frames = Some(0)` 指定により出力順序が入力順序と一致することが保証される。既存の `test_roundtrip_force_keyframe` は `keyframe_count >= 2` の緩い検証だが、本テストでは「先頭 1 枚のみ KEY」の厳密検証を行う)
3. **`KeyframeMode::Disabled` 下での `force_keyframe` 動作確認** (関数名例: `test_keyframe_mode_disabled_with_force_keyframe`)
   - テスト 2 と同様に config を再構築 + `g_lag_in_frames = Some(0)` を明示
   - 30 フレームを encode、ただし中盤 (例えば `i == 10`) のみ `EncodeOptions { force_keyframe: true }` を渡し、他フレームは `EncodeOptions { force_keyframe: false }`
   - `EncodedFrame::is_keyframe()` で先頭フレームと `i == 10` フレームの 2 枚のみキーフレームであり、他の 28 枚は非キーフレームであることを厳密に assert する (既存 `test_roundtrip_force_keyframe` との差分は「Disabled 指定下でも force_keyframe が効くこと」「他フレームが厳密に非 KEY であること」の確認)

libaom 側から set した値を read-back する形の検証は本 issue では行わない。`Encoder::new` 通過と roundtrip / PSNR / `is_keyframe` 判定による間接検証のみとする。

### 4. CHANGES.md

`## develop` セクション直下 (`### misc` は使わない) に以下のエントリを追加する。種別の順序 (UPDATE → ADD → CHANGE → FIX) を守り、担当者行は 2 文字インデント。既存の `[ADD] Encoder::reconfigure ...` エントリの後ろに並べる。`KeyframeMode` への `#[non_exhaustive]` 付与と `Disabled` variant 追加は同一コミット同一 PR で出るが、利用者から見て独立した破壊変更なので `[CHANGE]` を 2 エントリに分けて記載する。

```
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
```

## 実装タスク

1. `EncoderConfig` に 7 フィールドを追加し、`EncoderConfig::new()` の初期化と `Encoder::apply_controls()` を拡張する (設計 §1)
2. `KeyframeMode` を改修する: `#[non_exhaustive]` を付与し、`Disabled` variant を追加し、既存 `Fixed` に `#[deprecated]` 属性と修正された rustdoc を付与する。`Encoder::init()` の kf_mode マッピングを 3 値対応に拡張する (設計 §2)
3. `tests/test_roundtrip.rs` にテスト 3 件を追加する (設計 §3)
4. `cargo test` / `cargo clippy --all-targets --all-features -- -D warnings` / `cargo fmt -- --check` が通ることを確認する
5. `CHANGES.md` を更新する (設計 §4)

## スコープ外

- `ReconfigureParams` への追加 (本 issue で追加するフラグはいずれも midstream 更新が不要)
- SVC ランタイム制御 (`AV1E_SET_SVC_PARAMS`)。`pending/0013-enh-add-svc-runtime-control.md` で別扱い
- realtime 配信向けのプリセット / ヘルパー (例: `EncoderConfig::realtime()` 等)
- `*_cost_upd_freq` フィールドの enum 化 (例: `enum CostUpdateFreq { SuperBlock, SuperBlockRow, Tile, Off }`)。既存 `aq_mode` 等と同じく `u32` で受ける流儀に揃え、値域チェックは libaom 側に委譲する
- `KeyframeMode::Fixed` variant の削除 (`#[deprecated]` で当面残す。削除する場合は別 issue で扱う)
- `examples/` への追加。realtime プロファイル用 example が必要になった場合は別 issue を立てる
- `set_control` の C 型 (`c_int` vs `c_uint`) の整理。既存 `enable_keyframe_filtering` 等の `c_uint` パラメータも現状 `c_int` で渡している。型不整合の解消はスコープ外

## 参考

- libaom control 群: `AV1E_SET_ENABLE_ORDER_HINT` / `AV1E_SET_ENABLE_REF_FRAME_MVS` / `AV1E_SET_ENABLE_ANGLE_DELTA` / `AV1E_SET_INTRA_DEFAULT_TX_ONLY` / `AV1E_SET_COEFF_COST_UPD_FREQ` / `AV1E_SET_MODE_COST_UPD_FREQ` / `AV1E_SET_MV_COST_UPD_FREQ`
- libaom kf_mode 定数: `aom_kf_mode_AOM_KF_DISABLED` / `aom_kf_mode_AOM_KF_FIXED` (deprecated alias) / `aom_kf_mode_AOM_KF_AUTO`
