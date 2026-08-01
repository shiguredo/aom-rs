# 画像フォーマットと g_profile の整合を Encoder::init で検証する

- Created: 2026-08-02
- Completed: {YYYY-MM-DD}
- Branch: feature/fix-validate-image-format-profile
- Polished: {YYYY-MM-DD}
- Reporter: @voluntas

## 目的

デフォルト設定 (g_profile = 0) のまま I422 / I444 を指定すると `Encoder::new` が成功した後に 1 枚目の `Encoder::encode` で必ず失敗する「init 成功 → 遅延失敗」の踏み方を解消する。init 時に明確なエラーを返すべきである。

## 現状

libaom の `validate_img` (libaom `av1/av1_cx_iface.c`) は以下を要求するが、aom-rs の `Encoder::init` は検証しない:

- I422 / I42216 は `g_profile == 2` 必須
- I444 / I44416 は `g_profile != 0` 必須 (monochrome でない場合)

`EncoderConfig::new` のデフォルトは `g_profile = 0` のため、デフォルト構成 + I422 / I444 は `Encoder::new` が成功し、初回 `encode` で `AOM_CODEC_INVALID_PARAM` になる。エラーが出るタイミングが遅く、原因の特定が難しい。

## 設計方針

`Encoder::init` で `image_format` × `g_profile` の組み合わせを事前検証し、`Error::with_reason` で分かりやすいエラーを返す。

16-bit 修正 (issue 0029) と関連する: I42216 / I44416 は profile 制約と HIGHBITDEPTH フラグの両方が必要になるため、対応順序を 0029 と揃える。

## 完了条件

- 不正な組み合わせ (例: デフォルト g_profile 0 で I422) で `Encoder::new` が明確なエラーを返すこと
- 正しい組み合わせ (例: I422 + g_profile 2) でエンコードが成功すること

## 解決方法

- `Encoder::init` に image_format × g_profile の事前検証を追加する
- フォーマット別 roundtrip テストを追加する (各フォーマットに必要な g_profile をテストヘルパーに持たせる)
