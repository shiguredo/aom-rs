# EncoderConfig に fixed_qp_offsets を追加する

Created: 2026-03-25
Model: Opus 4.6

## 概要

libaom の `aom_codec_enc_cfg_t` に含まれる `fixed_qp_offsets` フィールド (`[c_int; 5]`) が未実装。

## 根拠

EncoderConfig は libaom の `aom_codec_enc_cfg_t` のフィールドを網羅する方針だが、このフィールドは libaom 側で deprecated 扱いになっている。

## pending の理由

libaom のソースコードに以下のコメントがある:

> Deprecated and ignored. DO NOT USE.
> TODO(aomedia:3269): Remove fixed_qp_offsets in libaom v4.0.0.

libaom v4.0.0 で削除予定のため、追加しても将来的に破壊的変更が必要になる。libaom v4 のリリース状況を見て判断する。
