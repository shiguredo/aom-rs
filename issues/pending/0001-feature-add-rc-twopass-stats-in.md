# EncoderConfig に rc_twopass_stats_in / rc_firstpass_mb_stats_in を追加する

Created: 2026-03-25
Model: Opus 4.6

## 概要

libaom の `aom_codec_enc_cfg_t` に含まれる以下の 2 フィールドが未実装。

- `rc_twopass_stats_in` (`aom_fixed_buf_t`)
- `rc_firstpass_mb_stats_in` (`aom_fixed_buf_t`)

## 根拠

EncoderConfig は libaom の `aom_codec_enc_cfg_t` のフィールドを網羅する方針だが、この 2 フィールドは `aom_fixed_buf_t` (生ポインタ + サイズ) であり、安全な Rust API として公開するには設計判断が必要。

## pending の理由

`aom_fixed_buf_t` は以下の構造:

```c
typedef struct aom_fixed_buf {
    void *buf;
    size_t sz;
} aom_fixed_buf_t;
```

生ポインタを含むため、安全に公開するには:

1. `&[u8]` スライスで受け取り内部でコピーする方式
2. ライフタイム付き参照で借用する方式
3. `Vec<u8>` で所有権を持つ方式

いずれの方式にするか、また 2 パスエンコーディングのワークフロー全体 (1 パス目の stats 出力 → 2 パス目の stats 入力) をどう設計するかの判断が必要。
