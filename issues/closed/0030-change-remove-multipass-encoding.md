# EncodingPass を OnePass のみに削減してマルチパスエンコードを非対応とする

- Created: 2026-08-02
- Completed: 2026-08-06
- Branch: feature/change-multipass-encoding
- Polished: 2026-08-06
- Reporter: @voluntas

## 目的

公開 API の `EncodingPass` (FirstPass / SecondPass / ThirdPass) が機能しない状態を解消する。機能しない API を公開したままにすると、ユーザーは init 失敗や出力ゼロをライブラリのバグと誤認する。

## 現状

- `Encoder::next_frame` は非フレームパケット (`AOM_CODEC_STATS_PKT`) を黙って破棄するため、`EncodingPass::FirstPass` では 1 パス目の統計データが失われ、エンコード結果が一切得られない
- `EncodingPass::SecondPass` / `ThirdPass` は libaom が init 時に `rc_twopass_stats_in.buf` の設定を要求する (libaom `av1/av1_cx_iface.c` の検証) が、`EncoderConfig` に設定手段がないため必ず `Encoder::new` が失敗する
- 現状 API で実際に機能するのは `EncodingPass::OnePass` のみ

また、`EncodingPass::ThirdPass` は libaom の 3 パスモード (`CONFIG_THREE_PASS`、デフォルト 0) が有効なビルドと、2 パス目のビットストリームを 3 パス目へ渡す入力手段が必要であり、aom-rs の現行ビルドでは 3 パス実装本体がコンパイル対象外となる。

## 設計方針

`EncodingPass` から FirstPass / SecondPass / ThirdPass を削除し、OnePass のみとする。`EncoderConfig::g_pass` フィールドは残す (OnePass のみ選択可能。`None` の場合は libaom のデフォルト `AOM_RC_ONE_PASS` が使われる)。OnePass variant と g_pass フィールドを残すのは、将来マルチパスを追加する際の拡張点を残すためである。

需要と実装コストを考慮した結果、マルチパスエンコードの実装 (stats パケットの公開 + `rc_twopass_stats_in` の設定手段) は行わない。FirstPass / SecondPass のみを実装する案も 2 パス全体の基盤実装を要し、ThirdPass は libaom のビルド構成変更 (`CONFIG_THREE_PASS=1`) と 2 パス目ビットストリームの入力 API が必要で実装コストが非常に高い。マルチパスエンコードの需要も限定的であるため、マルチパス実装全体を見送る。

本変更は公開 API の削除を伴う後方互換のない変更であるため、CHANGES.md には `[CHANGE]` として記録する。`EncoderConfig` は libaom の `aom_codec_enc_cfg_t` のフィールドを網羅する方針 (pending issue 0001 / 0003 の根拠) だが、機能しない API を公開しない方針を優先する。

`Encoder::next_frame` のパケット種別フィルタリングは変更しない (FirstPass を削除するため、`AOM_CODEC_STATS_PKT` の破棄は問題にならない)。

pending issue 0001 (`rc_twopass_stats_in` / `rc_firstpass_mb_stats_in` の追加) はマルチパス実装を前提としているため、本 issue の対応後は不要になる。0001 は本 issue のスコープ外として pending のまま残す (将来マルチパスを実装する場合は reopened にして検討する)。

## 完了条件

- `EncodingPass` の variant が OnePass のみであること (FirstPass / SecondPass / ThirdPass が存在しないこと)
- `EncoderConfig::g_pass` フィールドが残存していること
- src/lib.rs の `EncodingPass` enum と `EncoderConfig::g_pass` の rustdoc が OnePass のみの内容に更新されていること
- README と skills/shiguredo-aom/SKILL.md に FirstPass / SecondPass / ThirdPass の記述が残っておらず、シングルパスのみ対応と明記されていること
- CHANGES.md の `## develop` に `[CHANGE]` エントリが追加されていること
- 既存のテストが全て通ること (回帰なし)

## 解決方法

- src/lib.rs の `EncodingPass` enum から FirstPass / SecondPass / ThirdPass を削除し、OnePass のみにした。enum と `EncoderConfig::g_pass` の rustdoc も OnePass のみの内容に更新した (シングルパスのみ対応・拡張点として variant を残す旨・`None` と `Some(OnePass)` が同一挙動になる旨を明記)
- `Encoder::init` の g_pass マッピングを OnePass のみに簡素化した (将来のマルチパス variant 追加に備える拡張点として、OnePass のみのマッピングをコメント付きで残す)
- `Encoder::next_frame` のパケット種別フィルタリングは変更していない (設計方針どおり)
- README の `EncodingPass` の説明と `g_pass` の説明を OnePass のみに更新し、シングルパスのみ対応と明記した (拡張点として残す理由も追記)
- skills/shiguredo-aom/SKILL.md の `EncodingPass` の記述を OnePass のみに更新し、シングルパスのみ対応と明記した
- CHANGES.md の `## develop` に `[CHANGE]` エントリを追記した
- tests/roundtrip.rs に `g_pass: Some(EncodingPass::OnePass)` を明示指定する回帰テストを追加した (None デフォルトと同じ挙動になるが、明示指定のコードパスを固定する)
