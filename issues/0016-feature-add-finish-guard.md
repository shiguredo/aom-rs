# finish() 後の encode() / decode() / reconfigure() にガードを追加する

Created: 2026-07-21
Model: Qwen Code

## 概要

`Encoder` / `Decoder` に `finished` フラグが存在しないため、`finish()` 後に `encode()` / `decode()` / `reconfigure()` / 2 回目の `finish()` を呼ぶと、終端処理済みのコーデックに再度データが投入され、未定義動作が発生する。

## 根拠

`Encoder` 構造体（`src/lib.rs:1336-1344`）:

```rust
pub struct Encoder {
    ctx: sys::aom_codec_ctx,
    cfg: sys::aom_codec_enc_cfg,
    img: sys::aom_image,
    iter: sys::aom_codec_iter_t,
    frame_count: usize,
    image_format: ImageFormat,
    plane_sizes: PlaneSizes,
}
```

`finished` フラグがない。`check_iter_drained`（`src/lib.rs:2270-2280`）は `iter` が非 NULL の場合のみガードし、「ストリームが終了した」ことを一切追跡していない。

`finish()`（`src/lib.rs:2258-2268`）は `aom_codec_encode` に NULL img を渡してストリーム終端を通知する。その後 `encode()` を呼ぶと、`check_iter_drained` は `iter == null` なので通過し、終端処理済みのエンコーダーに再度有効な img を渡す。

libaom の `aom_codec_encode` は NULL img による flush 後に非 NULL img を受け取った場合の動作を定義していない。

`Decoder` も同様の問題を持つ（`src/lib.rs:148-151`）。

## 修正方針

`Encoder` / `Decoder` に `finished: bool` フィールドを追加する。

- `finish()` 呼び出し時に `finished = true` を設定する
- `encode()` / `decode()` / `reconfigure()` の冒頭で `finished` をチェックし、`true` の場合はエラーを返す
- 2 回目の `finish()` もエラーを返す

## テスト戦略

- `finish()` 後に `encode()` を呼ぶとエラーになるテスト
- `finish()` 後に `decode()` を呼ぶとエラーになるテスト
- `finish()` 後に `reconfigure()` を呼ぶとエラーになるテスト
- 2 回目の `finish()` がエラーになるテスト

## 後方互換

`finish()` 後の呼び出しがエラーになるため、既存のコードが `finish()` 後に `encode()` を呼んでいる場合は動作が変わる。ただし、それは未定義動作を引き起こすコードであり、エラーで止める方が正しい。
