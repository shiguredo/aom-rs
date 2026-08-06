# 画像フォーマットと g_profile / g_bit_depth の整合を Encoder::init で検証する

- Created: 2026-08-02
- Completed: {YYYY-MM-DD}
- Branch: feature/fix-validate-image-format-profile
- Polished: 2026-08-06
- Reporter: @voluntas

## 目的

画像フォーマット (ImageFormat) × g_profile × g_bit_depth の不整合を `Encoder::init` で明確なエラーとして検出し、「init 成功 → 遅延失敗」の踏み方を解消する。

## 現状

libaom は画像フォーマットと profile / ビット深度の整合を encode 時に検証する (libaom `av1/av1_cx_iface.c` の validate_img、`av1/encoder/encoder.c` の av1_receive_raw_frame)。aom-rs の `Encoder::init` は検証しないため、`Encoder::new` が成功した後に初回 `encode` で `AOM_CODEC_INVALID_PARAM` になる組み合わせが存在する。

libaom の検証条件は次のとおり:

- I422 / I42216 は `g_profile == 2` 必須 (validate_img。monochrome 例外なし)
- I444 / I44416 は `g_profile != 0` 必須 (monochrome でない場合。validate_img)
- profile 0 は非モノクロで 4:2:0 のみ許可、profile 1 は 4:4:4 のみ許可、profile 2 はビット深度 10 以下で 4:2:2 のみ許可 (av1_receive_raw_frame)

`EncoderConfig::new` のデフォルトは `g_profile = 0`、`g_bit_depth = None` (= 8) のため、デフォルト構成 + I422 / I444 は `Encoder::new` が成功し、初回 `encode` で `AOM_CODEC_INVALID_PARAM` になる。エラーが出るタイミングが遅く、原因の特定が難しい。

## 設計方針

`Encoder::init` で `image_format` × `g_profile` × 有効ビット深度の組み合わせを事前検証し、不正な組み合わせは `Error::with_reason` で `AOM_CODEC_INVALID_PARAM` を返す (libaom の encode 時検証と同じエラーコード)。

有効ビット深度は次のとおり定義する:

- 16-bit フォーマット (I42016 / I42216 / I44416): `g_bit_depth` (None なら 8)
- 8-bit フォーマット: 常に 8 (g_bit_depth の指定は判定に使わない。8-bit フォーマット + `g_bit_depth > 8` は HIGHBITDEPTH フラグなしで libaom が init 時に明確なエラーを返すため、それ以上の検証は行わない)

有効な組み合わせは次のとおり (libaom の validate_img / av1_receive_raw_frame の検証と AV1 仕様の profile 定義による。16-bit フォーマットは有効ビット深度 8 でも 10 でも同じ profile になる):

- 有効ビット深度 8〜10: I420 / Yv12 / Nv12 / I42016 は profile 0、I422 / I42216 は profile 2、I444 / I44416 は profile 1
- 有効ビット深度 12: 全フォーマット profile 2 のみ (profile < 2 は libaom が init 時に拒否するため、事前検証の対象外)

monochrome: `monochrome: Some(true)` の場合は I444 / I44416 + profile 0 を許可する (libaom の validate_img / av1_receive_raw_frame の monochrome 例外に合わせる)。monochrome 例外が適用されるのは profile 0 のみで、profile 2 の 4:2:2 制約には例外がない。I422 系は monochrome 例外がないため常に profile 2 必須。profile 1 + monochrome は libaom が init 時に拒否するため、事前検証の対象外。

16-bit フォーマットの正のラウンドトリップテストは、issue 0029 (HIGHBITDEPTH フラグ修正) のマージ後でないと通らない (フラグ欠如で encode が失敗する)。`g_bit_depth > 8` の指定を含むテスト (12-bit 等) も、0029 マージ前は libaom の init 自体が `g_bit_depth > 8` かつ HIGHBITDEPTH フラグなしで失敗するため、0029 の後に実施する。検証ロジック自体は 0029 に依存しない。

## 完了条件

- 不正な組み合わせ (判定表の全セルのうち不正な組み合わせ。代表例) で `Encoder::new` が明確なエラーを返すこと:
  - I422 + `g_profile: 0` (デフォルト)
  - I444 + `g_profile: 0` (デフォルト)
  - I444 + `g_profile: 2` (8-bit。profile 2 + ビット深度 10 以下は 4:2:2 のみ許可のため)
  - I420 + `g_profile: 1` (profile 1 は 4:4:4 のみ許可のため)
  - I420 + `g_profile: 2` (8-bit)
  - Yv12 + `g_profile: 1`
  - I444 + `g_profile: 2` + `monochrome: Some(true)` (monochrome 例外は profile 0 のみ)
- 正しい組み合わせ (代表例) でエンコードが成功すること:
  - I422 + `g_profile: 2`
  - I444 + `g_profile: 1`
  - I420 + `g_profile: 0` (デフォルト)
  - I444 + `g_profile: 0` + `monochrome: Some(true)`
- 12-bit (I42016 + `g_bit_depth: Some(12)` + `g_profile: 2`) が誤って拒否されないこと (issue 0029 の後に実施)
- 16-bit フォーマットの正しい組み合わせ (例: I42016 + `g_bit_depth: Some(10)` + `g_profile: 0`) が 0031 の検証で誤って拒否されないこと (issue 0029 の後に実施)
- 既存のテストが全て通ること (回帰なし)

## 解決方法

- `Encoder::init` に `image_format` × `g_profile` × 有効ビット深度の事前検証を追加する
- 検証の回帰テストを追加する (不正な組み合わせが init でエラーになること、正しい組み合わせが成功すること。判定表の全セルをカバーする。検証対象外のセル: 8-bit フォーマット + `g_bit_depth > 8`、12-bit + profile < 2、profile 1 + monochrome は libaom の init エラーを期待する)
- テストヘルパーはフォーマット × ビット深度から有効な profile を導出する関数とし、issue 0029 / 0021 の実装時にも再利用できる形にする
