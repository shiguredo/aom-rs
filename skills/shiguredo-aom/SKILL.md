---
name: shiguredo-aom
description: 時雨堂の libaom (AV1) Rust バインディング shiguredo_aom の機能・API リファレンス。AV1 エンコーダー / デコーダー、ImageFormat、EncoderConfig (aom_codec_enc_cfg_t / aom_codec_control 準拠)、レート制御、タイル、スーパーレゾリューション、S-Frame、midstream reconfigure、prebuilt / source-build に関する質問時に使用。
---

# shiguredo_aom

時雨堂が公開している [libaom](https://aomedia.googlesource.com/aom) ベースの AV1 エンコーダー / デコーダー Rust バインディング。

## バージョン情報

- crate 名: `shiguredo_aom`
- バージョン: 2026.2.0-canary.2
- libaom バージョン: v3.14.1
- Rust Edition: 2024
- 最小 Rust バージョン: 1.93
- ライセンス: Apache-2.0
- リポジトリ: <https://github.com/shiguredo/aom-rs>

シンボルは `shiguredo_aom_` プレフィックス付きで書き換えられており、他の libaom ベースのライブラリと同一プロセス内に同居しても衝突しない。

## ビルド

### デフォルト (prebuilt バイナリ)

```bash
cargo build
```

GitHub Releases から prebuilt バイナリをダウンロードしてリンクする。対応プラットフォーム:

- Ubuntu 26.04 / 24.04 / 22.04 (x86_64, arm64)
- macOS 26 / 15 (arm64)
- Windows 11 / Windows Server 2025 (x86_64)

`LIBAOM_TARGET` 環境変数で prebuilt バイナリのプラットフォーム名を明示指定できる。

### ソースビルド

```bash
cargo build --features source-build
```

追加要件: Git、C コンパイラ、NASM。

```bash
# Ubuntu
sudo apt-get install -y build-essential nasm
# macOS
brew install nasm
# Windows
choco install nasm
```

### docs.rs 向け

libaom がない環境ではドキュメント生成のみ可能。

```bash
DOCS_RS=1 cargo doc --no-deps
```

## ビルドメタデータ定数

```rust
pub const BUILD_REPOSITORY: &str; // 参照した libaom リポジトリ URL
pub const BUILD_VERSION: &str;    // libaom のバージョン (タグ)
```

## エラー型

```rust
pub struct Error { /* code, function, reason, detail */ }
impl std::fmt::Display for Error { /* "<fn>() failed: code=<n>, reason=..., detail=..." */ }
impl std::error::Error for Error {}
```

`code` は libaom の `aom_codec_err_t`、`function` は失敗した内部関数名、`reason` / `detail` は libaom が返す説明。

## コーデック情報照会

```rust
pub fn supported_codecs() -> CodecInfo;

pub struct CodecInfo {
    pub codec: VideoCodecType,        // Av1
    pub decoding: DecodingInfo,
    pub encoding: EncodingInfo,
}

pub struct DecodingInfo {
    pub supported: bool,
    pub hardware_accelerated: bool,   // 常に false (libaom はソフトウェア実装)
}

pub struct EncodingInfo {
    pub supported: bool,
    pub hardware_accelerated: bool,   // 常に false
    pub profiles: EncodingProfiles,
}

pub enum EncodingProfiles {
    Av1(Vec<Av1EncodingProfile>),
    Unsupported,
}

pub enum Av1EncodingProfile {
    Profile0, // 8/10-bit 4:2:0
    Profile1, // 8/10-bit 4:4:4
    Profile2, // 8/10/12-bit 4:2:0, 4:2:2, 4:4:4
}
```

## 画像フォーマット

```rust
pub enum ImageFormat {
    I420,    // YUV 4:2:0 planar (Y, U, V)
    Yv12,    // YUV 4:2:0 planar (Y, V, U)
    Nv12,    // YUV 4:2:0 semi-planar (Y, UV interleaved)
    I422,    // YUV 4:2:2 planar
    I444,    // YUV 4:4:4 planar
    I42016,  // YUV 4:2:0 planar 16-bit
    I42216,  // YUV 4:2:2 planar 16-bit
    I44416,  // YUV 4:4:4 planar 16-bit
}

pub enum ImageData<'a> {
    I420   { y: &'a [u8], u: &'a [u8], v: &'a [u8] },
    Yv12   { y: &'a [u8], u: &'a [u8], v: &'a [u8] },
    Nv12   { y: &'a [u8], uv: &'a [u8] },
    I422   { y: &'a [u8], u: &'a [u8], v: &'a [u8] },
    I444   { y: &'a [u8], u: &'a [u8], v: &'a [u8] },
    I42016 { y: &'a [u8], u: &'a [u8], v: &'a [u8] },
    I42216 { y: &'a [u8], u: &'a [u8], v: &'a [u8] },
    I44416 { y: &'a [u8], u: &'a [u8], v: &'a [u8] },
}
```

`I42016` などの 16-bit 系は、各プレーンを `&[u8]` として渡す（リトルエンディアン 16-bit サンプルがバイト列として格納されている前提）。

## デコーダー

### DecoderConfig

```rust
pub struct DecoderConfig {
    pub threads: Option<u32>,           // None なら 1
    pub w: Option<u32>,                 // 幅のヒント
    pub h: Option<u32>,                 // 高さのヒント
    pub allow_lowbitdepth: Option<bool>,
}

impl DecoderConfig {
    pub fn new() -> Self;
}
impl Default for DecoderConfig { /* new と同等 */ }
```

すべての設定が `None` の場合は libaom のデフォルトが使われる。

### Decoder

```rust
impl Decoder {
    pub fn new(config: DecoderConfig) -> Result<Self, Error>;
    pub fn decode(&mut self, data: &[u8]) -> Result<(), Error>;
    pub fn finish(&mut self) -> Result<(), Error>;
    pub fn next_frame(&mut self) -> Option<DecodedFrame<'_>>;
}
unsafe impl Send for Decoder {}
```

`decode()` または `finish()` の後は `next_frame()` を `None` が返るまで呼び続ける必要がある。`next_frame()` を呼び切らずに次の `decode()` / `finish()` を呼ぶとエラーになる。

### DecodedFrame

```rust
impl DecodedFrame<'_> {
    pub fn format(&self) -> Result<ImageFormat, Error>;
    pub fn is_high_depth(&self) -> bool;
    pub fn y_plane(&self) -> Result<&[u8], Error>;
    pub fn u_plane(&self) -> Result<&[u8], Error>;
    pub fn v_plane(&self) -> Result<&[u8], Error>;
    pub fn y_stride(&self) -> Result<usize, Error>;
    pub fn u_stride(&self) -> Result<usize, Error>;
    pub fn v_stride(&self) -> Result<usize, Error>;
    pub fn width(&self) -> usize;
    pub fn height(&self) -> usize;
}
```

ビットストリームプロファイルに応じて I420 / I422 / I444 (および各 16-bit 版) のいずれかが返る。フォーマット変換は行われない。

## エンコーダー

### Usage / RateControlMode / KeyframeMode / EncodingPass

```rust
pub enum Usage {
    GoodQuality,  // 品質と速度のバランス
    Realtime,     // 最も高速 (WebRTC 等のリアルタイム用途)
    AllIntra,     // 全 I フレーム
}

pub enum RateControlMode {
    Vbr, Cbr, Cq, Q,
}

pub enum KeyframeMode {
    Disabled, // 自動配置を停止 (force_keyframe で挿入)。シーケンス先頭は常に KEY
    Fixed,    // [deprecated] AOM_KF_FIXED は AOM_KF_DISABLED の deprecated alias
    Auto,     // 自動配置 (kf_min_dist == kf_max_dist で auto_key が無効化される)
}

pub enum EncodingPass {
    OnePass, FirstPass, SecondPass, ThirdPass,
}

pub struct AomRational { pub num: i32, pub den: i32 }

pub enum SuperblockSize { Size64x64, Size128x128, Dynamic }
pub enum ContentType    { Default, Screen, Film }
```

### EncoderConfig

`EncoderConfig` のフィールドは libaom の `aom_codec_enc_cfg_t` と `aom_codec_control` 制御パラメータに対応する。`Option` のフィールドは `None` で libaom デフォルトを採用する。

新規フィールド追加は破壊的変更として扱う。`EncoderConfig::new()` で生成してから個別フィールドを上書きする運用。

```rust
let mut config = EncoderConfig::new(g_w, g_h, image_format);
```

主要フィールド（カテゴリ別の代表のみ。詳細は README）:

- 基本: `image_format`, `g_usage`, `g_threads`, `g_profile`, `g_w`, `g_h`, `g_bit_depth`, `g_input_bit_depth`, `g_timebase`, `g_pass`, `g_lag_in_frames`, `g_error_resilient`, `g_limit`, `g_forced_max_frame_width`, `g_forced_max_frame_height`
- レート制御: `rc_end_usage`, `rc_target_bitrate` (kbps), `rc_min_quantizer`, `rc_max_quantizer`, `rc_undershoot_pct`, `rc_overshoot_pct`, `rc_buf_sz`, `rc_buf_initial_sz`, `rc_buf_optimal_sz`, `rc_dropframe_thresh`, `rc_2pass_vbr_*`
- リサイズ / スーパーレゾリューション: `rc_resize_mode`, `rc_resize_denominator`, `rc_resize_kf_denominator`, `rc_superres_mode`, `rc_superres_denominator`, `rc_superres_kf_denominator`, `rc_superres_qthresh`, `rc_superres_kf_qthresh`, `enable_superres`
- キーフレーム / GOP: `kf_mode`, `kf_min_dist`, `kf_max_dist`, `fwd_kf_enabled`, `sframe_dist`, `sframe_mode`
- タイル: `tile_columns`, `tile_rows`, `tile_width_count`, `tile_height_count`, `tile_widths`, `tile_heights`, `large_scale_tile`
- 速度 / 品質: `cpu_used` (0-10, 大きいほど高速), `cq_level`, `sharpness`, `static_threshold`, `arnr_max_frames`, `arnr_strength`, `max_intra_bitrate_pct`, `aq_mode`, `deltaq_mode`, `lossless`, `noise_sensitivity`, `intra_default_tx_only`
- フィルター: `enable_cdef`, `enable_restoration`, `loopfilter_control`, `enable_obmc`, `denoise_noise_level`, `denoise_block_size`, `film_grain_test_vector`
- モーション / 予測: `enable_global_motion`, `enable_warped_motion`, `enable_tpl_model`, `enable_keyframe_filtering`, `min_gf_interval`, `max_gf_interval`, `gf_max_pyramid_height`, `max_reference_frames`
- Intra: `enable_filter_intra`, `enable_smooth_intra`, `enable_paeth_intra`, `enable_cfl_intra`, `enable_palette`, `enable_intrabc`
- パーティション: `superblock_size`, `enable_rect_partitions`, `enable_ab_partitions`, `enable_1to4_partitions`
- カラー: `color_primaries`, `transfer_characteristics`, `matrix_coefficients`, `color_range`, `enable_chroma_deltaq`, `enable_dual_filter`
- realtime 配信向け sequence-level 機能フラグ: `enable_order_hint`, `enable_ref_frame_mvs`, `enable_angle_delta` (libaom 通常ビルドのデフォルトは `1` で、`Usage::Realtime` を選んでも変わらない。realtime 配信用途では `Some(false)` を指定する)
- realtime 配信向け RD コスト更新頻度: `coeff_cost_upd_freq`, `mode_cost_upd_freq`, `mv_cost_upd_freq` (有効値 `0..=3`、`0=SB`, `1=SB row`, `2=tile`, `3=off`。libaom 通常ビルドのデフォルトは `0` で、realtime 配信用途では `Some(3)` を指定する)
- その他: `row_mt`, `tune_content`, `monochrome`, `full_still_picture_hdr`, `save_as_annexb`, `use_fixed_qp_offsets`

### EncodeOptions / ReconfigureParams

```rust
pub struct EncodeOptions {
    pub force_keyframe: bool,
}

#[derive(Default)]
pub struct ReconfigureParams {
    pub rc_target_bitrate: Option<u32>, // kbps
}
```

`ReconfigureParams` は `Encoder::reconfigure()` でランタイムに変更可能なフィールドのみを保持する。libwebrtc の `LibaomAv1Encoder::SetRates` 相当のビットレート切替に使う。

### Encoder

```rust
impl Encoder {
    pub fn new(config: EncoderConfig) -> Result<Self, Error>;
    pub fn reconfigure(&mut self, params: ReconfigureParams) -> Result<(), Error>;
    pub fn encode(&mut self, image: &ImageData<'_>, options: &EncodeOptions) -> Result<(), Error>;
    pub fn finish(&mut self) -> Result<(), Error>;
    pub fn next_frame(&mut self) -> Option<EncodedFrame<'_>>;
}
```

`encode()` / `finish()` の後は `next_frame()` を `None` が返るまで呼び続ける。

### EncodedFrame

```rust
impl EncodedFrame<'_> {
    pub fn data(&self) -> Result<&[u8], Error>;
    pub fn is_keyframe(&self) -> bool;
}
```

## コード例

### コーデック対応情報の照会

```rust
use shiguredo_aom::{supported_codecs, EncodingProfiles};

let info = supported_codecs();
println!("decoding supported: {}", info.decoding.supported);
println!("encoding supported: {}", info.encoding.supported);

if let EncodingProfiles::Av1(profiles) = info.encoding.profiles {
    for profile in profiles {
        println!("profile: {:?}", profile);
    }
}
```

### エンコード (リアルタイム / CBR)

```rust
use shiguredo_aom::{
    AomRational, EncodeOptions, Encoder, EncoderConfig,
    ImageData, ImageFormat, RateControlMode, Usage,
};

let mut config = EncoderConfig::new(1920, 1080, ImageFormat::I420);
config.g_usage = Usage::Realtime;
config.rc_end_usage = RateControlMode::Cbr;
config.rc_target_bitrate = 4000; // kbps
config.cpu_used = Some(8);
config.g_threads = Some(4);
config.g_timebase = AomRational { num: 1, den: 30 };

let mut encoder = Encoder::new(config)?;

let image = ImageData::I420 { y: &y_data, u: &u_data, v: &v_data };
encoder.encode(&image, &EncodeOptions { force_keyframe: false })?;

while let Some(frame) = encoder.next_frame() {
    let data = frame.data()?;
    let is_key = frame.is_keyframe();
    // ...
}

encoder.finish()?;
while let Some(frame) = encoder.next_frame() {
    // 残フレームのフラッシュ
}
```

### Midstream reconfigure (ビットレート切替)

タイムベースは初期化時に固定し、ランタイムでは `reconfigure()` で `rc_target_bitrate` のみ変える。エンコーダーは破棄しない。

```rust
use shiguredo_aom::ReconfigureParams;

encoder.reconfigure(ReconfigureParams {
    rc_target_bitrate: Some(2000), // kbps
})?;
```

完全な例は `examples/midstream_reconfigure.rs` を参照。

### キーフレーム強制

```rust
encoder.encode(&image, &EncodeOptions { force_keyframe: true })?;
```

### デコード

```rust
use shiguredo_aom::{Decoder, DecoderConfig};

let mut decoder = Decoder::new(DecoderConfig::default())?;
decoder.decode(&compressed_data)?;

while let Some(frame) = decoder.next_frame() {
    let format = frame.format()?;
    let w = frame.width();
    let h = frame.height();
    let y = frame.y_plane()?;
    let y_stride = frame.y_stride()?;
    // u_plane / v_plane / u_stride / v_stride / is_high_depth ...
}

decoder.finish()?;
while let Some(frame) = decoder.next_frame() {
    // 残フレームのフラッシュ
}
```

## 落とし穴 / 注意事項

- `Encoder::encode()` / `Decoder::decode()` および `finish()` の後は、必ず `next_frame()` を `None` まで呼び切ること。残ったままで次回 `encode` / `decode` / `finish` を呼ぶとエラーになる。
- `g_timebase` は初期化時に固定し、ランタイムでフレームレートに合わせて変更しない (libaom の典型的な前提に揃える)。
- `rc_target_bitrate` の単位は kbps。
- 16-bit フォーマット (`I42016` / `I42216` / `I44416`) のプレーンは `&[u8]` 渡し。
- libaom はソフトウェアエンコーダーであり、`hardware_accelerated` は常に `false`。
- prebuilt バイナリ非対応プラットフォームでは `source-build` feature を有効にし、NASM を用意する。
- `Decoder` と `Encoder` はどちらも `Send`（`unsafe impl Send`）。スレッド境界を越えて移動できるが、`&mut self` による排他アクセスが必要。`Sync` は実装されていない（共有参照での並行アクセスは不可）。

## 関連ファイル

- README: `/Users/voluntas/shiguredo/aom-rs/README.md` (フィールド全表)
- 公開 API: `/Users/voluntas/shiguredo/aom-rs/src/lib.rs`
- 例: `/Users/voluntas/shiguredo/aom-rs/examples/midstream_reconfigure.rs`
- 変更履歴: `/Users/voluntas/shiguredo/aom-rs/CHANGES.md`
