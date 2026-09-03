# 2 パス VBR パラメータの扱いを決定する

- Created: 2026-08-06
- Completed: {YYYY-MM-DD}
- Branch: 決定結果に応じて実装 issue で確定する
- Polished: 2026-08-12

## 目的

シングルパスのみ対応 (0030 で `EncodingPass` を OnePass のみに削減) とした一方、`EncoderConfig` に `rc_2pass_*` フィールドが残ったままの状態の扱いを決定する。決定後の実装 (削除 or 注記追加) は決定結果に応じた別 issue で行う。

## 現状

`EncoderConfig` に以下の 3 フィールドが残っている (src/lib.rs の `EncoderConfig`・README.md のレート制御テーブル・skills/shiguredo-aom/SKILL.md):

- `rc_2pass_vbr_bias_pct`
- `rc_2pass_vbr_minsection_pct`
- `rc_2pass_vbr_maxsection_pct`

libaom v3.14.1 の実装における実効性はフィールドごとに異なる:

- `rc_2pass_vbr_bias_pct` は 2 パスモード専用 (av1/encoder/pass2_strategy.c でのみ使用)。OnePass のみ対応の現状では実効性がない。validate_config が 0-100 の範囲チェックを行う (av1/av1_cx_iface.c の `RANGE_CHECK_HI(cfg, rc_2pass_vbr_bias_pct, 100)`)
- `rc_2pass_vbr_minsection_pct` / `rc_2pass_vbr_maxsection_pct` は OnePass VBR でも実効する。`av1_rc_update_framerate` が `vbrmin_section` / `vbrmax_section` から `min_frame_bandwidth` / `max_frame_bandwidth` を算出し (av1/encoder/ratectrl.c)、OnePass VBR のフレームターゲット計算 (`av1_calc_pframe_target_size_one_pass_vbr` / `av1_calc_iframe_target_size_one_pass_vbr`) が `clamp_pframe_target_size` / `clamp_iframe_target_size` でフレームあたりのビットレート上下限として使う。validate_config の範囲チェックは存在しない

0030 の対応では「機能しない API を公開しない方針を優先する」としたが、削除対象は `EncodingPass` の variant のみで、2 パス VBR パラメータには踏み込まなかった。一方で「`EncoderConfig` は libaom の `aom_codec_enc_cfg_t` のフィールドを網羅する方針 (pending issue 0001 / 0003 の根拠)」も存在する。0030 の判断を 3 フィールドに適用するかを決める必要があるが、上記のとおり実効性はフィールドごとに異なる。

## 設計方針

- 3 フィールドを一括で扱わず、実効性の違いを考慮してフィールドごとの扱いを決定する
- 判断基準: 0030 は「機能しない API を公開しない方針を優先する」と決定済みであり、本 issue はその決定を各フィールドに適用するかどうかを判断する。`rc_2pass_vbr_bias_pct` は実効性がないため削除が既定となるが、`rc_2pass_vbr_minsection_pct` / `rc_2pass_vbr_maxsection_pct` は OnePass VBR でも機能するため「機能しない API」に該当せず、`aom_codec_enc_cfg_t` のフィールド網羅方針 (pending issue 0001 / 0003) の観点で残す判断が自然である
- 判断材料 (拡張点論・0004 の将来検討言及) は既定判断を覆す根拠として検討する。適用して既定判断を覆す場合は、その理由を本 issue の解決方法に記録する
- 0030 は `g_pass` フィールドと OnePass variant を「将来マルチパスを追加する際の拡張点」として残した。この判断を 2 パス VBR パラメータにも適用するかどうかも判断材料に含める。また、closed issue 0004 の解決方法で `rc_2pass_vbr_maxsection_pct` を max_bitrate 相当の将来検討対象としている点も判断材料に含める
- 決定後の実装は本 issue では行わず、決定内容に応じて実装 issue を別途起票する (削除なら change、注記追加なら update)

## pending の理由

2 パスエンコードは当面使用しないため、本 issue の対応 (3 フィールドの扱いの決定と実装) は保留する。3 フィールドは現状のまま残し、実害はない (bias_pct は OnePass では実効性がないが、validate_config の範囲チェックのみで安全)。2 パスエンコードが必要になった時点で reopened にして、決定と実装を行う。

## 完了条件

- 3 フィールドそれぞれの扱い (削除 or 注記追加) が決定され、決定内容と判断根拠が本 issue の解決方法に記録されていること
- 決定内容に応じた実装 issue が起票されていること

## 解決方法

- 決定内容と判断根拠をここに追記する
