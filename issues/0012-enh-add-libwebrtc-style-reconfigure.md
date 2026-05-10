# libwebrtc と同等のランタイム再設定パターンを実現する

Created: 2026-05-10
Model: Opus 4.7

## 概要

libwebrtc の AV1 エンコーダーラッパー (`modules/video_coding/codecs/av1/libaom_av1_encoder.cc`) が行っている「エンコーダーを destroy せずに `aom_codec_enc_config_set()` でビットレート・解像度・SVC 層構成を動的に変更する」運用パターンを、aom-rs ユーザーが安全に再現できるように API と規約を整備する。

具体的には、現状の `Encoder::reconfigure()` だけでは表現できない以下の領域を埋める。

1. SVC（Scalable Video Coding）ランタイム制御 (`AV1E_SET_SVC_PARAMS` と `layer_target_bitrate[]`)
2. 「総ビットレートを先に更新してから SVC パラメータを更新する」という順序制約のドキュメント化と API 上のガード
3. libwebrtc が採用している「timebase は固定し、フレームレートはエンコード時の duration（PTS 差分）で表現する」運用例の提示

## 根拠

WebRTC / SFU 用途で aom-rs を組み込む場合、libwebrtc の運用パターンと整合できることが事実上の要件になる。libwebrtc 実装の調査で以下が判明している（`webrtc/src/modules/video_coding/codecs/av1/libaom_av1_encoder.cc` 1202-1252 行）。

- ビットレート変更時に `aom_codec_destroy` は呼ばず、`cfg_.rc_target_bitrate` 更新 → `aom_codec_enc_config_set()` だけで済ませる
- `AOME_SET_TARGET_BITRATE` / `AOME_SET_BITRATE` のような単独 control コードは使わない
- SVC を使う場合は `AV1E_SET_SVC_PARAMS` を `aom_codec_control` で別途渡し、`layer_target_bitrate[sid * num_temporal + tid]` に累積ビットレート（kbps）を詰める
- ソースコメントで明示されている順序制約：
  > The bitrates calculated internally in libaom when `AV1E_SET_SVC_PARAMS` is called depends on the currently configured `rc_target_bitrate`. If the total target bitrate is not updated first a division by zero could happen.
- `g_timebase` は `{1, 90000}`（RTP の 90kHz）で固定し、フレームレート変動はエンコーダーの内部状態（speed controller など）で扱う。`g_timebase` は reconfigure で動かさない

aom-rs 側の現状はこのパターンの一部しかカバーできていない:

- SVC ランタイム制御 API がない（`AV1E_SET_SVC_PARAMS` のバインディングを公開していない）
- `ReconfigureParams` には `g_w` / `g_h` / `g_timebase` / `rc_target_bitrate` が並列に並ぶだけで、libwebrtc が守っている順序制約や「timebase は触らない」という規約が伝わらない（`g_w` / `g_h` / `g_timebase` の削除自体は `issues/0006` で扱う）

## 設計

### 1. SVC ランタイム制御 API を追加する

libaom の `aom_svc_params_t` をラップする型を公開し、`Encoder::set_svc_params()` を追加する。

```rust
/// SVC（Scalable Video Coding）パラメータ
///
/// `AV1E_SET_SVC_PARAMS` で libaom に渡す層構成。
/// `layer_target_bitrate[sid * number_temporal_layers + tid]` には
/// 「spatial id = sid かつ temporal id <= tid」のフレームの累積ビットレート (kbps) を入れる。
#[derive(Debug, Clone)]
pub struct SvcParams {
    pub number_spatial_layers: u32,
    pub number_temporal_layers: u32,
    pub max_quantizers: [u32; AOM_MAX_LAYERS],
    pub min_quantizers: [u32; AOM_MAX_LAYERS],
    pub scaling_factor_num: [u32; AOM_MAX_SS_LAYERS],
    pub scaling_factor_den: [u32; AOM_MAX_SS_LAYERS],
    pub layer_target_bitrate: [u32; AOM_MAX_LAYERS],
    pub framerate_factor: [u32; AOM_MAX_TS_LAYERS],
}

impl Encoder {
    /// SVC 層構成を設定する（`AV1E_SET_SVC_PARAMS` 相当）
    ///
    /// 呼び出し前に `reconfigure()` で `rc_target_bitrate` を最新の総ビットレートに
    /// 更新しておかないと、libaom 内部でゼロ除算が発生する可能性がある（libwebrtc 実装より）。
    pub fn set_svc_params(&mut self, params: &SvcParams) -> Result<(), Error>;
}
```

定数（`AOM_MAX_LAYERS` 等）は `sys` 経由で公開するか、ラッパー側で再エクスポートする。

### 2. 順序制約を API で表現する

「総ビットレート → SVC パラメータ」という順序を doc コメントに書くだけでは伝わらない懸念がある。少なくとも以下のいずれかを採る:

- 案 A: `Encoder::reconfigure_with_svc(ReconfigureParams, Option<&SvcParams>)` を追加し、内部で「rc_target_bitrate 更新 → `aom_codec_enc_config_set` → `AV1E_SET_SVC_PARAMS`」を一括で行う
- 案 B: `set_svc_params` 単体は残しつつ、`set_svc_params` 実行時に「直近の `reconfigure()` から総ビットレートが整合しているか」を検証する（合計値 vs `cfg.rc_target_bitrate`）

実装時に案 A を第一候補としつつ、レビューで決める。

### 3. `g_timebase` の扱いを libwebrtc 方式に寄せる

`ReconfigureParams::g_timebase` の削除自体は `issues/0006-bug-remove-gw-gh-from-reconfigure.md` で扱う。本 issue では 0006 適用後に「libwebrtc と同様、timebase は初期化時に固定（典型値 `{1, 90000}`）し、フレームレート変動は PTS の duration で表現する」という運用ガイドを doc / example に追加する。

### 4. ドキュメント / 例

- `Encoder::reconfigure` および新規 `Encoder::set_svc_params` の doc コメントに、libwebrtc 実装での運用例を引用する形で順序制約と destroy しない方針を明記する
- `examples/` または doctest として、ビットレートと SVC 構成を midstream で更新するサンプルを 1 本追加する

## 実装タスク

1. `sys` レイヤーに `aom_svc_params_t` 関連定数（`AOM_MAX_LAYERS`, `AOM_MAX_SS_LAYERS`, `AOM_MAX_TS_LAYERS`）と `AV1E_SET_SVC_PARAMS` を露出させる
2. `SvcParams` 型を `src/lib.rs` に追加する（`Default` は libaom の単一層構成相当）
3. `Encoder::set_svc_params()` を実装する
4. 案 A を採るなら `Encoder::reconfigure_with_svc()` を追加する
5. doc コメントに libwebrtc 実装由来の順序制約と destroy しない方針を明記する
6. PBT / unittest を追加する
   - 単一層構成（1 spatial × 1 temporal）で `set_svc_params` → エンコード完走
   - 多層構成（例: 1 spatial × 3 temporal）で `reconfigure` 後に `set_svc_params` → エンコード完走
   - 順序逆転（先に `set_svc_params` を呼んでから `reconfigure` で総ビットレートを下げる）でもクラッシュしないこと
7. `examples/` または doctest を 1 本追加する
8. `CHANGES.md` の `## develop` に `[ADD]` で追記する

## スコープ外

- フレームレート再設定 API（timebase 変更）は本 issue では追加しない。0006 で `ReconfigureParams::g_timebase` ごと削除する方針（libwebrtc 方式優先）
- `aom_codec_destroy` → `aom_codec_enc_init` を伴うフル再初期化パスは追加しない（libwebrtc も `Release()` 経路のみ）
- libaom の他の control コード（`AOME_SET_CPUUSED` 等）の動的変更 API はここで扱わない（必要になった時点で別 issue）

## 参考

- libwebrtc `LibaomAv1Encoder::SetRates` (`webrtc/src/modules/video_coding/codecs/av1/libaom_av1_encoder.cc` 1202-1252)
- libwebrtc v2 解像度変更パス (`libaom_av1_encoder_v2.cc` 669-689)
- libaom `aom_codec_enc_config_set`, `aom_codec_control(AV1E_SET_SVC_PARAMS, ...)`
- 既存実装 `Encoder::reconfigure` (`src/lib.rs` 1993-2019)
- 関連 issue: `0006-bug-remove-gw-gh-from-reconfigure.md`（`g_w` / `g_h` / `g_timebase` 削除を含む）
