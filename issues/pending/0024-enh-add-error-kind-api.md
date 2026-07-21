# Error 型にプログラム的なエラー種別判定手段を追加する

Created: 2026-07-21
Model: Qwen Code

## 概要

`Error` 型の全フィールドが private で、ユーザーがプログラム的にエラー種別を判定する手段がない。`code()` アクセサまたは `ErrorKind` enum の導入を検討する。

## 根拠

`src/lib.rs:21-27`:

```rust
pub struct Error {
    code: sys::aom_codec_err_t,
    function: &'static str,
    reason: Option<&'static str>,
    detail: Option<String>,
}
```

全フィールドが private。`code()` のようなパブリックアクセサも、`PartialEq` 実装も、エラー種別を表す enum も存在しない。

ユーザーは `Display` の文字列をパースする以外にエラー種別を判定する手段がない。`AOM_CODEC_CORRUPT_FRAME`（リトライ可能）と `AOM_CODEC_INVALID_PARAM`（呼び出し側のバグ）をプログラム的に区別できない。

### 関連する問題

`Error::with_reason` が `AOM_CODEC_ERROR` を汎用エラーとして濫用している（`src/lib.rs:216,240,310,352,360,368,375,405,2279,2341,2349`）。iter 未 drain、フォーマット不一致、plane ポインタ NULL など、libaom 由来でない Rust 側の検証エラーにも一律 `AOM_CODEC_ERROR` を割り当てている。

## 修正方針（設計判断が必要）

選択肢:
1. `code()` アクセサを追加する（最小変更）
2. `ErrorKind` enum を導入し、`Error` に `kind()` メソッドを追加する（より構造的）
3. `AOM_CODEC_ERROR` の濫用を解消し、Rust 側検証エラーには `AOM_CODEC_INVALID_PARAM` を使う

設計判断を伴うため、本 issue は pending とする。

## 後方互換

公開 API の追加であり、後方互換はある。
