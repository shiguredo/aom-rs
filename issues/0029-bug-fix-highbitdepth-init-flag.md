# 16-bit エンコード時に AOM_CODEC_USE_HIGHBITDEPTH フラグを立てる

- Created: 2026-08-02
- Completed: {YYYY-MM-DD}
- Branch: feature/fix-highbitdepth-init-flag
- Polished: 2026-08-06
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

`Encoder::init` で 16-bit フォーマット (I42016 / I42216 / I44416) を指定した場合のみ `AOM_CODEC_USE_HIGHBITDEPTH` を init フラグに付与する。8-bit フォーマットのままフラグを立てると逆に 8-bit エンコードが全滅するため、必ず条件付きで立てる。

`g_bit_depth > 8` を 8-bit フォーマットと併用した場合はフラグを立てない。フラグを立てると init は成功するが encode が毎回失敗する (libaom `aom/src/aom_encoder.c` の aom_codec_encode が画像フォーマットの HIGHBITDEPTH ビットと init フラグの一致を要求する) 遅延失敗になるためである。フラグを立てなければ libaom が init 時に `AOM_CODEC_INVALID_PARAM` で明確なエラーを返す (修正前と同じ挙動)。この組み合わせの事前検証の追加は本 issue のスコープ外とする (issue 0031 は image_format × g_profile が対象であり、image_format × g_bit_depth は含まない)。

16-bit フォーマットを使用する場合は `g_bit_depth` (10 または 12) と `g_profile` の設定も必要である:

- `g_bit_depth`: 16-bit 入力のピクセル値は `1 << g_bit_depth` 未満に制限される (libaom の validate_hbd_input、デフォルト有効)。`g_bit_depth` が 8 のままでは 10-bit 値域のデータがエラーになる
- `g_profile`: フォーマット × ビット深度の有効な組み合わせは次のとおり (libaom の validate_img / encoder の profile 検証と AV1 仕様の profile 定義)
  - I42016 + 10-bit は profile 0 (profile 2 + 10-bit は 4:2:2 のみ許可のため不可)
  - I42216 + 10-bit は profile 2
  - I44416 + 10-bit は profile 1 (profile 0 不可、profile 2 + 10-bit は 4:2:2 のみ許可のため不可)
  - 12-bit は profile 2 のみ (profile < 2 は 12-bit 不可)
- `g_input_bit_depth`: libaom の validate_config が `g_input_bit_depth > g_bit_depth` をエラーにするため、aom-rs 側での追加の事前検証は行わない

init フラグの修正で `sys::AOM_CODEC_USE_HIGHBITDEPTH` 定数を参照するようになるため、docs.rs 用のダミー bindings (build.rs の DOCS_RS 分岐) に定数を追加する必要がある。ただし、DOCS_RS 分岐のダミー bindings は現在も完全性が不足している (issue 0022 で完全性検証が未完了のまま closed されている) ため、本 issue ではフラグ修正に必要な定数の追加のみを行い、ダミー全体の完全性修復はスコープ外とする。

## 完了条件

- I42016 + `g_bit_depth: Some(10)` + `g_profile: 0` で `Encoder::new` → `encode` → `finish` → デコードが成功すること
- I42216 + `g_bit_depth: Some(10)` + `g_profile: 2` で同様に成功すること
- I44416 + `g_bit_depth: Some(10)` + `g_profile: 1` で同様に成功すること
- I42016 + `g_bit_depth: Some(12)` + `g_profile: 2` で同様に成功すること
- デコード結果のフォーマットが入力と同じ 16-bit フォーマットであること (8-bit に落ちていないこと)
- 既存の 8-bit テストが全て通ること (回帰なし)

## 解決方法

- 設計方針のとおり `Encoder::init` で init フラグを条件付きで付与する (16-bit フォーマット指定時のみ)
- build.rs の DOCS_RS 分岐のダミー bindings に `AOM_CODEC_USE_HIGHBITDEPTH` 定数を追加する (ダミー全体の完全性修復はスコープ外)
- 16-bit のラウンドトリップテストを追加する (16-bit カラーバー生成ヘルパー + 16-bit 対応 PSNR + 16-bit 対応の Y プレーン抽出ヘルパー)
- 修正と同一 PR に回帰テストを含めること。16-bit 系のラウンドトリップは本 issue の回帰テストとして実装し、テストヘルパーは issue 0021 の実装時にも再利用できる形にする
- README に 16-bit フォーマットの利用条件 (`g_bit_depth` / `g_profile` の設定が必須であること) を追記する
