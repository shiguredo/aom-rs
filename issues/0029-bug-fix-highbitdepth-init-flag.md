# 16-bit エンコード時に AOM_CODEC_USE_HIGHBITDEPTH フラグを立てる

- Created: 2026-08-02
- Completed: {YYYY-MM-DD}
- Branch: feature/fix-highbitdepth-init-flag
- Polished: {YYYY-MM-DD}
- Reporter: @voluntas

## 目的

公開している 16-bit フォーマット (ImageFormat::I42016 / I42216 / I44416) と g_bit_depth (10, 12) が 100% 失敗するバグを修正する。README で対応を公開している機能が動作しない状態は正式リリースのブロッカーである。

## 現状

`Encoder::init` が `aom_codec_enc_init_ver` に常にフラグ 0 を渡しており、`AOM_CODEC_USE_HIGHBITDEPTH` を一度も立てない。

- `g_bit_depth: Some(10|12)` を指定すると `Encoder::new` 自体が `AOM_CODEC_INVALID_PARAM` で失敗する (libaom `aom/src/aom_encoder.c` の init 検証: `g_bit_depth > 8` かつ HIGHBITDEPTH フラグなしで拒否)
- 16-bit フォーマットを指定すると init は成功するが、`Encoder::encode` が毎回 `AOM_CODEC_INVALID_PARAM` で失敗する (libaom が画像フォーマットの HIGHBITDEPTH ビットと init フラグの不一致を拒否)

この libaom は `CONFIG_AV1_HIGHBITDEPTH=1` でビルドされている (build.rs の CMake 定義) ため、フラグさえ立てれば動作するはずの機能が封印されている。デコーダー側 (DecodedFrame) は 16-bit デコードに対応済みで問題ない。

テストは I420 のみで 16-bit 系が一切カバーされておらず、このバグはテストで検出されていない。

## 設計方針

`Encoder::init` で 16-bit フォーマット (I42016 / I42216 / I44416) または `g_bit_depth > 8` を指定した場合のみ `AOM_CODEC_USE_HIGHBITDEPTH` を init フラグに付与する。8-bit フォーマットのままフラグを立てると逆に 8-bit エンコードが全滅するため、必ず条件付きで立てる。

あわせて以下を確認する:

- `g_bit_depth` / `g_input_bit_depth` の整合 (libaom は `g_input_bit_depth > g_bit_depth` をエラーにする)
- 16-bit フォーマット × g_profile の制約 (I42216 は profile 2 必須、I44416 は profile 0 不可。issue 0031 と関連)

## 完了条件

- I42016 / I42216 / I44416 で `Encoder::new` → `encode` → `finish` → デコードが成功すること
- `g_bit_depth: Some(10)` / `Some(12)` で `Encoder::new` が成功すること
- 既存の 8-bit テストが全て通ること (回帰なし)

## 解決方法

- init フラグの条件付き付与
- 10-bit のラウンドトリップテスト追加 (16-bit カラーバー生成ヘルパー + 16-bit 対応 PSNR)
- 修正と同一 PR に回帰テストを含めること
- 修正後は prebuilt 配布物の bindings.rs に `AOM_CODEC_USE_HIGHBITDEPTH` 定数が含まれることを確認する
