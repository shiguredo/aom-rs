# Encoder 側の unsafe コードに DecodedFrame と同等の防御的検証を追加する

- Created: 2026-07-31
- Completed: {YYYY-MM-DD}
- Branch: feature/fix-encoder-unsafe-validation
- Polished: 2026-07-31
- Model: Qwen Code qwen3.8-max-preview

## 目的

Encoder 側の unsafe コード（`plane_sizes` 計算と `encode()` 内の `from_raw_parts_mut`）に、`DecodedFrame::plane` と同等の防御的検証を追加してメモリ安全の非対称を解消する。

## 現状

`DecodedFrame::plane` (`src/lib.rs` の `DecodedFrame::plane` メソッド) には以下の検証がある:

- `checked_mul` によるオーバーフローチェック
- `isize::MAX` 上限チェック
- planes ポインタの null チェック
- stride の正値チェック

一方、Encoder 側にはこれらの検証がない:

1. `Encoder::init` 内の `plane_sizes` 計算 (`src/lib.rs` の `Encoder::init` 内 `plane_sizes` match 式) は `height * img.stride[0] as usize` を `checked_mul` なしで計算している。release モードでオーバーフローするとラップし、`encode()` のサイズ検証を偶然通過した際に `from_raw_parts_mut` で実際のバッファと異なる長さのスライスが作られる
2. `Encoder::encode` 内の `from_raw_parts_mut` (`src/lib.rs` の `Encoder::encode` 内 unsafe ブロック) は `self.img.planes[0..2]` が null でないことを前提にしているが検証していない

`aom_img_alloc` の成功は null でないことを強く示唆するが、unsafe コードの健全性は「強く示唆」ではなく「保証」でなければならない。

## 設計方針

- `Encoder::init` の `plane_sizes` 計算に `checked_mul` と `isize::MAX` チェックを追加する
- `aom_img_alloc` 成功直後に planes ポインタの null チェックを追加する（`encode()` 側の毎回チェックではなく、初期化時の 1 回チェックで十分。`img` は Encoder のライフタイム中に再割り当てされないため）
- stride の正値チェックも初期化時に追加する
- 検証失敗時は `aom_img_free` + `aom_codec_destroy` でリソースを解放してからエラーを返す（`img` 割り当て成功後の検証であるため、既存の `aom_img_alloc` 失敗パスより 1 つ解放対象が多い）

## 完了条件

- `plane_sizes` 計算が `checked_mul` + `isize::MAX` チェック付きになる
- `aom_img_alloc` 成功後に planes の null チェックと stride 正値チェックが入る
- 既存の全テストが通過する
- `cargo clippy --workspace --all-targets --features source-build -- -D warnings` が通過する

## 解決方法

- `Encoder::init` 内の `plane_sizes` match 式の各 arm で `checked_mul` を使い、`None` の場合はエラーを返す。`isize::MAX` 上限チェックも同様に行う
- `aom_img_alloc` の null チェック直後（`img.assume_init()` の後）に、使用するプレーン数に応じた null チェックと stride 正値チェックを追加する:
  - ThreePlanes 系（I420, Yv12, I422, I444, I42016, I42216, I44416）: `img.planes[0]`, `img.planes[1]`, `img.planes[2]` の null チェックと `img.stride[0]`, `img.stride[1]`, `img.stride[2]` の正値チェック
  - TwoPlanes 系（Nv12）: `img.planes[0]`, `img.planes[1]` の null チェックと `img.stride[0]`, `img.stride[1]` の正値チェック
- 検証失敗時は `aom_img_free` でイメージバッファを解放し、`aom_codec_destroy` で ctx を解放してから `Err` を返す（既存の `aom_img_alloc` 失敗パスは `img` 未割り当てのため `aom_codec_destroy` のみだが、本検証ポイントは `img` 割り当て成功後であるため両方の解放が必要）
