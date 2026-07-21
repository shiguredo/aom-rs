# YV12 フォーマットで U/V プレーンが入れ替わる + aom_img_alloc 失敗時の ctx リーク

Created: 2026-07-21
Model: Qwen Code

## 概要

2 つの致命的バグを修正する。

1. `ImageData::Yv12` でエンコードすると U/V プレーンが入れ替わり、出力映像の色が破壊される
2. `Encoder::init` で `aom_img_alloc` が失敗した場合、初期化済みのコーデックコンテキストがリークする

## 根拠

### バグ 1: YV12 U/V 入れ替わり

`src/lib.rs:2218-2228`:

```rust
ImageData::I420 { y, u, v }
| ImageData::Yv12 { y, u, v }  // ← I420 と同じ arm
| ImageData::I422 { y, u, v }
// ...
=> {
    std::slice::from_raw_parts_mut(self.img.planes[0], y.len()).copy_from_slice(y);
    std::slice::from_raw_parts_mut(self.img.planes[1], u.len()).copy_from_slice(u);
    std::slice::from_raw_parts_mut(self.img.planes[2], v.len()).copy_from_slice(v);
}
```

libaom の `AOM_IMG_FMT_YV12` は `planes[0]=Y, planes[1]=V, planes[2]=U` の順でプレーンを配置する。しかしコードは `u`（フィールド doc: "U プレーン"、`src/lib.rs:480`）を `planes[1]`（libaom 上の V プレーン）に、`v`（フィールド doc: "V プレーン"、`src/lib.rs:482`）を `planes[2]`（libaom 上の U プレーン）にコピーしている。

4:2:0 では U/V のサイズが同一のため、サイズ検証（`src/lib.rs:2195-2210`）では検出できない。YV12 のテストが存在しないため未検出。

### バグ 2: aom_img_alloc 失敗時の ctx リーク

`src/lib.rs:1591-1604`:

```rust
let code = sys::aom_codec_enc_init_ver(ctx.as_mut_ptr(), iface, &aom_config, 0, ...);
Error::check(code, "aom_codec_enc_init_ver", None)?;
// ↑ 成功すると ctx は初期化済み

let img_ptr = sys::aom_img_alloc(img.as_mut_ptr(), img_fmt, ...);
if img_ptr.is_null() {
    return Err(Error::with_reason(...));
    // ↑ ctx は初期化済みだが aom_codec_destroy が呼ばれない
    // ctx はローカル変数の MaybeUninit であり、Drop は走らない
}
```

`ctx` は `Self` 構築前（`src/lib.rs:1651`）のローカル `MaybeUninit<sys::aom_codec_ctx>`。`MaybeUninit` には Drop 実装がないため、`aom_codec_destroy` が呼ばれず libaom 内部リソースが永久にリークする。`src/lib.rs:1660` のコメント「これ以降の操作に失敗しても ctx は Drop によって確実に解放される」は `Self` 構築後のみ正しい。

## 再現手順

### バグ 1

1. `ImageFormat::Yv12` で `Encoder` を作成する
2. U プレーンに 0、V プレーンに 255 を設定した `ImageData::Yv12` をエンコードする
3. デコード結果の U/V プレーンが入れ替わっている（U=255, V=0 になっている）

### バグ 2

1. OOM 等の条件下で `aom_img_alloc` が NULL を返す
2. `Encoder::new` が `Err` を返すが、libaom のコーデックコンテキストが解放されない

## 修正方針

### バグ 1

`ImageData::Yv12` の match arm を分離し、`planes[1]` に `v`（V データ）、`planes[2]` に `u`（U データ）をコピーする:

```rust
ImageData::Yv12 { y, u, v } => {
    std::slice::from_raw_parts_mut(self.img.planes[0], y.len()).copy_from_slice(y);
    std::slice::from_raw_parts_mut(self.img.planes[1], v.len()).copy_from_slice(v);
    std::slice::from_raw_parts_mut(self.img.planes[2], u.len()).copy_from_slice(u);
}
```

### バグ 2

`aom_img_alloc` 失敗時に `aom_codec_destroy` を呼んでから `Err` を返す:

```rust
if img_ptr.is_null() {
    sys::aom_codec_destroy(&mut ctx.assume_init());
    return Err(Error::with_reason(...));
}
```

## テスト戦略

- YV12 のラウンドトリップテストを追加する（U/V プレーンの値が入れ替わらないことを検証）
- `aom_img_alloc` 失敗のテストは OOM の再現が困難なため、コードレビューで対応

## 後方互換

バグ 1 は YV12 のエンコード結果が修正される（色成分が正しくなる）。YV12 を使用している既存ユーザーの出力は変化するが、それはバグ修正であり後方互換の問題ではない。

バグ 2 は内部実装の修正のみ。公開 API の変更なし。
