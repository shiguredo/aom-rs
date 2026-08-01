# マルチパスエンコード (EncodingPass) の破綻を修正する

- Created: 2026-08-02
- Completed: {YYYY-MM-DD}
- Branch: feature/fix-multipass-encoding
- Polished: {YYYY-MM-DD}
- Reporter: @voluntas

## 目的

公開 API の `EncodingPass` (FirstPass / SecondPass / ThirdPass) が機能しない状態を解消する。機能しない API を公開したままにすると、ユーザーは init 失敗や出力ゼロをライブラリのバグと誤認する。

## 現状

- `Encoder::next_frame` は非フレームパケット (`AOM_CODEC_STATS_PKT`) を黙って破棄するため、`EncodingPass::FirstPass` では 1 パス目の統計データが失われ、エンコード結果が一切得られない
- `EncodingPass::SecondPass` / `ThirdPass` は libaom が init 時に `rc_twopass_stats_in.buf` の設定を要求する (libaom `av1/av1_cx_iface.c` の検証) が、`EncoderConfig` に設定手段がないため必ず `Encoder::new` が失敗する
- 現状 API で実際に機能するのは `EncodingPass::OnePass` のみ

## 設計方針

対応方針を確定する必要がある。

- 案 A: マルチパスを実装する (stats パケットの公開 + `rc_twopass_stats_in` の設定手段)。pending issue 0001 (rc_twopass_stats_in の追加) と整合させる
- 案 B: `EncodingPass` を `OnePass` のみに削減し、機能しない variant を公開しない

実装コストと需要を考慮して判断する。

## 完了条件

- `EncodingPass::FirstPass` / `SecondPass` / `ThirdPass` が実際にマルチパスエンコードとして機能する
- または、機能しない variant が公開 API から除去され、doc に OnePass のみ対応と明記される

## 解決方法

設計判断 (案 A / 案 B) を決めてから対応する。案 A の場合、`Encoder::next_frame` のパケット種別フィルタリングと `EncoderConfig` のフィールド追加が変更対象になる。
