# libwebrtc スタイルのランタイム再設定運用を整える

Created: 2026-05-10
Model: Opus 4.7

## 概要

libwebrtc の AV1 エンコーダーラッパー (`webrtc/src/modules/video_coding/codecs/av1/libaom_av1_encoder.cc`) が採用している運用パターン「エンコーダーを destroy せず `aom_codec_enc_config_set()` だけで動的に設定を変える」「`g_timebase` は初期化時に固定し、ランタイムでは動かさない」を、aom-rs の `Encoder::reconfigure` 周辺で利用者が自然に再現できるように doc とサンプルを整える。

SVC（1 エンコーダ多レイヤー）統合は別 issue (`pending/0013-enh-add-svc-runtime-control`) に切り出す。本 issue は SVC 抜きの「単一ストリーム向けランタイム再設定」のみを対象とする。

## 根拠

WebRTC / SFU で aom-rs を使う場合、libwebrtc と同じ運用パターンに合わせられることが事実上の要件になる。既存 `Encoder::reconfigure` は libwebrtc と同じく destroy せずに `aom_codec_enc_config_set` で更新する作りになっており、機能的にはほぼ揃っている。一方で、

- 「destroy しないのが推奨運用」だと doc から読み取れない
- 「timebase は固定運用」だと doc から読み取れない（0006 で `g_timebase` を削除しても、なぜ削除したかの背景が利用者に伝わらない）
- midstream でビットレートを更新する具体例が欠けている

ため、運用方針を文書化したい。

## 設計

### 1. `Encoder::reconfigure` の doc を libwebrtc 方式に揃える

`Encoder::reconfigure` の doc コメントに以下を明記する:

- ランタイム中はエンコーダーを破棄せず、本メソッドで設定を更新するのが推奨運用であること
- libwebrtc の `LibaomAv1Encoder::SetRates` と同じパターンであること
- `g_timebase` は初期化時に固定し（典型値 `{1, 90000}`、RTP の 90kHz）、フレームレート変動はエンコード時の duration（PTS 差分）で表現すること
- 将来 SVC 拡張（`AV1E_SET_SVC_PARAMS`）を追加する場合に「総ビットレートを先に更新してから SVC params を更新する」という順序制約があること（libwebrtc コードコメント由来）

### 2. midstream 再設定の example を追加する

`examples/` に「30fps エンコード途中でビットレートを切り替える」サンプルを 1 本追加する。timebase を固定したまま `Encoder::reconfigure` だけでビットレートを動的変更する典型パターンを示す。

### 3. SVC 拡張余地の記述

「将来の SVC 統合で `AV1E_SET_SVC_PARAMS` を扱う際は本メソッドの順序制約を踏襲する」旨を doc に残す。詳細設計は `pending/0013-enh-add-svc-runtime-control` で扱う。

## 実装タスク

1. `Encoder::reconfigure` の doc コメントを上記方針で書き直す（destroy しない方針 / timebase 固定運用 / 将来 SVC 拡張時の順序制約）
2. `examples/midstream_reconfigure.rs`（仮）を追加する
3. `CHANGES.md` の `## develop` に `[UPDATE]` 相当のエントリで doc 整備とサンプル追加を記載する

## 依存関係

- `0006-bug-remove-gw-gh-from-reconfigure` の適用後（`ReconfigureParams` から `g_w` / `g_h` / `g_timebase` 削除済み）に着手するのが望ましい。doc 文面が安定する
- `0011-misc-cleanup-reconfigure-code` の doc 整理と統合できる場合は同時に進めてよい

## スコープ外

- SVC ランタイム制御 API (`AV1E_SET_SVC_PARAMS`, `SvcParams`)。`pending/0013-enh-add-svc-runtime-control` で扱う
- 解像度動的変更（`g_w` / `g_h`）。aom-rs の現状実装では `plane_sizes` / `aom_image` バッファが追従しないため未対応。0006 で `ReconfigureParams` から削除する
- フレームレート動的変更（`g_timebase`）。本 issue の方針として「触らない」ため API 追加しない
- `aom_codec_destroy` → `aom_codec_enc_init` を伴うフル再初期化。libwebrtc も `Release()` 経路のみで採用していない

## 参考

- libwebrtc `LibaomAv1Encoder::SetRates` (`webrtc/src/modules/video_coding/codecs/av1/libaom_av1_encoder.cc` 1202-1252)
- libwebrtc v2 解像度変更パス (`libaom_av1_encoder_v2.cc` 669-689)
- libaom `aom_codec_enc_config_set`
- 既存実装 `Encoder::reconfigure` (`src/lib.rs` 1993-2019)
- 関連 issue: `0006-bug-remove-gw-gh-from-reconfigure.md`, `pending/0013-enh-add-svc-runtime-control.md`
