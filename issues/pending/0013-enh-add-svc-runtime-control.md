# SVC ランタイム制御 API を追加する

Created: 2026-05-10
Model: Opus 4.7

## pending 理由

SVC（1 エンコーダ多レイヤー）統合は API 設計の選択肢が複数残っているため、需要が固まるまで保留する。L1T1 Simulcast 用途は `Encoder::new()` を本数分インスタンス化することで対応可能であり、本 issue が無くても WebRTC / SFU の典型用途は成立する。

未確定の選択:

- API 形状: 案 A（`reconfigure` の `ReconfigureParams::svc` フィールドに統合）vs 案 B（`Encoder::set_svc_params()` 単体メソッドを追加）
- `SvcParams` のレイアウト: `Vec` 可変長 vs `[T; AOM_MAX_LAYERS]` 固定長
- 順序とロールバック: libwebrtc 順序（`AV1E_SET_SVC_PARAMS` → `aom_codec_enc_config_set`）を踏襲するか、逆順にして失敗時の中間状態を防ぐか
- 出力側拡張: `EncodedFrame` に `spatial_id` / `temporal_id` を露出する必要があるが、これは reconfigure とは別軸の API 拡張になる
- `SvcParams::validate()` の検証範囲（層数上限、Vec 長整合、累積単調性、`rc_target_bitrate` との整合）

## 概要

libwebrtc の AV1 エンコーダーラッパーが採用している SVC ランタイム制御パターン（`aom_codec_control(AV1E_SET_SVC_PARAMS, ...)`）を aom-rs に追加し、1 エンコーダで複数の spatial / temporal レイヤーを多重化したビットストリームを生成できるようにする。

## 根拠

WebRTC / SFU 用途で AV1 SVC を活用する場合、libaom の `AV1E_SET_SVC_PARAMS` を呼んで層構成（`number_spatial_layers`, `number_temporal_layers`, `layer_target_bitrate[]`, `scaling_factor_*`, `framerate_factor[]` 等）を設定する必要がある。aom-rs はこの API を露出していないため、SVC を使うアプリケーションは aom-rs を直接使えない。

libwebrtc 実装 (`webrtc/src/modules/video_coding/codecs/av1/libaom_av1_encoder.cc`) の SVC 制御パターン:

- `aom_svc_params_t` を `aom_codec_control(AV1E_SET_SVC_PARAMS, ...)` で渡す
- `layer_target_bitrate[sid * num_temporal + tid]` には「spatial=sid かつ temporal<=tid のフレームの累積ビットレート (kbps)」を入れる
- 順序制約（libwebrtc コードコメント由来）:
  > The bitrates calculated internally in libaom when `AV1E_SET_SVC_PARAMS` is called depends on the currently configured `rc_target_bitrate`. If the total target bitrate is not updated first a division by zero could happen.

## 設計案

### 案 A: `ReconfigureParams::svc` に統合

```rust
pub struct ReconfigureParams {
    pub rc_target_bitrate: Option<u32>,
    pub svc: Option<SvcParams>,
}
```

`reconfigure()` 内部で「rc_target_bitrate 更新 → AV1E_SET_SVC_PARAMS → aom_codec_enc_config_set」を一括実行。利用者には順序制約を意識させない。

### 案 B: `Encoder::set_svc_params()` を単独メソッドで追加

`reconfigure()` は触らず、SVC 専用の API として分離。`set_svc_params` 実行時に「直近の `cfg.rc_target_bitrate` と `layer_target_bitrate` の合計が整合しているか」を検証してエラーで弾く。

### `SvcParams` の型案

```rust
#[derive(Debug, Clone)]
pub struct SvcParams {
    pub number_spatial_layers: u8,   // 1..=AOM_MAX_SS_LAYERS (4)
    pub number_temporal_layers: u8,  // 1..=AOM_MAX_TS_LAYERS (8)

    /// 各空間レイヤーの解像度スケーリング (num, den)
    /// 長さ = number_spatial_layers
    pub scaling_factors: Vec<(u32, u32)>,

    /// レイヤー別累積ターゲットビットレート (kbps)
    /// 長さ = number_spatial_layers * number_temporal_layers
    /// インデックス = sid * number_temporal_layers + tid
    pub layer_target_bitrates: Vec<u32>,

    /// レイヤー別 min/max quantizer
    /// 長さ = number_spatial_layers * number_temporal_layers
    pub min_quantizers: Vec<u32>,
    pub max_quantizers: Vec<u32>,

    /// 時間レイヤー別フレームレート係数
    /// 長さ = number_temporal_layers
    pub framerate_factors: Vec<u32>,
}

impl SvcParams {
    /// 単一層構成（spatial=1, temporal=1）のヘルパー
    pub fn single_layer(target_bitrate_kbps: u32, qp_min: u32, qp_max: u32) -> Self;

    fn validate(&self) -> Result<(), Error>;
    fn to_aom(&self) -> sys::aom_svc_params_t;
}
```

### 出力側拡張

`Encoder::next_frame()` の戻り値（`EncodedFrame` 等）に `spatial_id` / `temporal_id` を露出する。SVC では 1 入力フレームに対して spatial layer 数だけパケットが取れるため、既存テストで暗黙に仮定している「1 入力 1 packet」の整合確認も必要。

## 実装タスク（着手時に確定する）

1. `sys` レイヤーに `AV1E_SET_SVC_PARAMS`, `aom_svc_params_t`, `AOM_MAX_LAYERS`, `AOM_MAX_SS_LAYERS`, `AOM_MAX_TS_LAYERS` を露出
2. `SvcParams` 型と `validate()` / `to_aom()` を追加
3. 案 A or 案 B を確定して API を実装
4. `EncodedFrame` に `spatial_id` / `temporal_id` を追加
5. PBT / unittest:
   - 単一層 `SvcParams` で reconfigure → エンコード完走
   - 1 spatial × 3 temporal → エンコード完走、各 packet の `temporal_id` が期待通り
   - 不正 `SvcParams`（層数オーバー、Vec 長不一致、累積非単調）で `validate()` がエラー
6. `examples/svc_encode.rs` を追加
7. `CHANGES.md` の `## develop` に `[ADD]` で追記

## 依存関係

- `0006-bug-remove-gw-gh-from-reconfigure` 適用後（`ReconfigureParams` から `g_w` / `g_h` / `g_timebase` 削除）
- `0010-enh-remove-reconfigure-default-derive` 適用後（`Default` 撤廃により `ReconfigureParams::svc` のデフォルトが `None` で済む）
- `0005-bug-fix-reconfigure-state-inconsistency` 適用後（cfg 複製による失敗ロールバックの仕組みに SVC 検証失敗も乗せる）
- `0012-enh-add-libwebrtc-style-reconfigure` 適用後（libwebrtc 方式の運用 doc / example が整っている前提で SVC を追加する）

## スコープ外

- Simulcast（独立エンコーダ複数本）。これは aom-rs では既に `Encoder::new()` を複数回呼べば実現可能なので追加 API 不要
- `aom_codec_destroy` → `aom_codec_enc_init` を伴うフル再初期化パス
- libaom の他の control コード（`AOME_SET_CPUUSED` 等）の動的変更 API

## 参考

- libwebrtc `LibaomAv1Encoder::SetRates` (`webrtc/src/modules/video_coding/codecs/av1/libaom_av1_encoder.cc` 1202-1252)
- libaom `aom_codec_control(AV1E_SET_SVC_PARAMS, ...)`, `aom_svc_params_t`
- 関連 issue: `0012-enh-add-libwebrtc-style-reconfigure.md`, `0006-bug-remove-gw-gh-from-reconfigure.md`
