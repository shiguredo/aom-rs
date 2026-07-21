# unsafe impl Send の安全性コメントと並行性の文書化を充実する

Created: 2026-07-21
Model: Qwen Code

## 概要

`unsafe impl Send` の安全性コメントが証明として不十分。Sync 非実装の根拠、複数インスタンスの並行使用、denoiser 有効時の挙動が未文書化。

## 根拠

### unsafe impl Send の安全性コメント不十分

`src/lib.rs:272-274`（Decoder）、`src/lib.rs:2313-2315`（Encoder）:

```rust
// 安全性: aom_codec_ctx はスレッド間で移動しても安全である。
// libaom の内部状態はスレッドローカルな資源に依存せず、
// コンテキストへの排他的アクセスがあれば（&mut self で保証される）問題ない。
unsafe impl Send for Decoder {}
```

以下の点が未記載:
- どのフィールドが `!Send` か（raw pointer 群）
- なぜ移動が安全か（ヒープ確保されたメモリはスレッドアフィニティを持たない）
- CONFIG_MULTITHREAD の前提

### Sync 非実装の根拠コメントがない

`Encoder` / `Decoder` は Send だが Sync ではない。これは正しい判断だが、なぜ Sync を実装しないかのコメントがない。

### denoiser 有効時の内部バッファ変更が未文書化

libaom の `aom_encoder.h` には「AV1E_SET_DENOISE_NOISE_LEVEL が非ゼロの場合、aom_codec_encode() は img->planes[i] を変更する」と明記されている。`encode()` の doc comment にこの挙動の記載がない。

### 複数インスタンスの並行使用に関する文書の欠如

複数の Encoder/Decoder インスタンスを別スレッドで並行に使用することは安全だが、どこにも記載がない。

## 修正方針

- 安全性コメントを構造化して書き換える
- Sync 非実装の根拠コメントを追加する
- `encode()` の doc comment に denoiser 挙動を追記する
- クレートレベルの doc comment に並行使用の安全性を追記する

## 後方互換

コメント・ドキュメントの修正のみ。コードの動作に変更なし。
