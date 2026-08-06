# 2 パス VBR パラメータの扱いを決定する

- Created: 2026-08-06
- Completed: {YYYY-MM-DD}
- Branch: feature/change-twopass-vbr-fields
- Polished: {YYYY-MM-DD}

## 目的

シングルパスのみ対応 (0030 で `EncodingPass` を OnePass のみに削減) とした一方、`EncoderConfig` に 2 パス VBR 専用パラメータが残ったままの状態の扱いを決定する。

## 現状

`EncoderConfig` に以下の 2 パス VBR 専用フィールドが残っている:

- `rc_2pass_vbr_bias_pct`
- `rc_2pass_vbr_minsection_pct`
- `rc_2pass_vbr_maxsection_pct`

これらのフィールドは libaom の 2 パスモード (g_pass == AOM_RC_SECOND_PASS) でのみ意味を持つ。OnePass のみの対応となった現状では、これらのフィールドは libaom の validate_config が範囲チェックするだけで実効性がない。

0030 の対応では「機能しない API を公開しない方針を優先する」としたが、削除対象は `EncodingPass` の variant のみで、2 パス VBR パラメータには踏み込まなかった。一方で「`EncoderConfig` は libaom の `aom_codec_enc_cfg_t` のフィールドを網羅する方針 (pending issue の根拠)」も存在し、両方針の整合を判断する必要がある。

## 設計方針

- 2 パス VBR パラメータを「削除する」「残す (シングルパスでは無効の注記付き)」「残す (現状のまま)」のいずれにするかを決定する
- 判断基準: 機能しない API を公開しない方針 (0030) と、aom_codec_enc_cfg_t のフィールド網羅方針 (pending issue) のどちらを優先するか
- フィールドを残す場合は、rustdoc と README に「シングルパスでは無効」の注記を追加する

## 完了条件

- 2 パス VBR パラメータの扱いが決定され、実装 (削除 or 注記追加) されていること
- 決定内容が CHANGES.md に記録されていること (削除する場合は [CHANGE]、注記追加の場合は misc)
- 既存のテストが全て通ること (回帰なし)

## 解決方法

- `EncoderConfig` の 2 パス VBR パラメータの扱いを決定し、実装する
- 関連する pending issue (EncoderConfig のフィールド網羅方針) との整合を確認する
