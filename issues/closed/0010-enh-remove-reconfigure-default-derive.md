# ReconfigureParams の Default derive を撤廃する

Created: 2026-05-10
Completed: 2026-05-10
Model: DeepSeek v4-pro

## 概要

`ReconfigureParams` に `#[derive(Default)]` が付与されており、全フィールド `None` のデフォルト値を生成する。これを `reconfigure()` に渡すと何も変更されないのに FFI 呼び出しが発生し、ユーザが誤って意図しない no-op を実行するリスクがある。

## 根拠

`ReconfigureParams::default()` は全フィールドが `None` になる。この値を `reconfigure()` に渡すと:

1. `self.cfg` のどのフィールドも変更されない
2. 同一設定で `aom_codec_enc_config_set()` が呼ばれる（無駄な FFI コール）
3. ユーザが `ReconfigureParams { rc_target_bitrate: Some(1000), ..Default::default() }` と書いた場合、明示的に指定したフィールドだけが変更されることが明確になるが、逆に `ReconfigureParams::default()` だけを渡すユースケースは実質存在しない

またプロジェクト内でスタイルが不統一である:
- `ReconfigureParams` → `#[derive(Default)]`
- `DecoderConfig` → 手動 `impl Default` → `Self::new()`
- `EncoderConfig` → `Default` 実装なし、`new()` のみ

## 修正方針

1. `ReconfigureParams` から `#[derive(Default)]` を外す
2. 必要に応じて手動 `Default` 実装または `new()` を追加する
3. `tests/test_roundtrip.rs` の `test_reconfigure_empty_params_is_noop` を削除する（`Default` がなくなればこのテストは不要になる）
4. 既存テストの `..Default::default()` 記述を、残りの全フィールドを明示的に `None` 指定する形に書き換える

### 代替: `Default` は残しつつ `new()` を追加

```rust
impl ReconfigureParams {
    pub fn new() -> Self {
        Self {
            g_w: None,
            g_h: None,
            g_timebase: None,
            rc_target_bitrate: None,
        }
    }
}
```

ただし全フィールド `None` で固定なら `Default` derive と実質変わらないため、撤廃が望ましい。

## 参考

- レビュー指摘: `feature/encoder-reconfigure` ブランチの `/review-diff-code` 結果より（設計指摘 6, 改善提案 5.1）

## 解決方法

当初は `#[derive(Default)]` を撤廃する方針で対応したが、`amf-rs` / `vpl-rs` / `nvcodec-rs` の姉妹クレートが全て `#[derive(Debug, Clone, Default)]` で `ReconfigureParams` を定義しており、`reconfigure(params: ReconfigureParams)` を所有権渡しで受ける形で揃っていることを踏まえ、クロスクレート統一を優先して `#[derive(Default)]` を残す方針に最終決定した。

- `ReconfigureParams` は `#[derive(Debug, Clone, Default)]` で定義する
- `Encoder::reconfigure(&mut self, params: ReconfigureParams)` も他クレートと同じく所有権渡し
- 外部からは `ReconfigureParams { rc_target_bitrate: Some(x), ..Default::default() }` でも `ReconfigureParams::default()` 経由でも構築できる

本 issue で当初懸念した「`default()` 単独呼び出しによる意図しない no-op」は、ドキュメントで「`Some` フィールドのみが書き換え対象」と明示する形で受容する。

## 振り返り

本 issue は **完全に空振り** した issue。当初撤廃した `#[derive(Default)]` を、その後のレビューで `#[non_exhaustive]` 付与 → 手動 `impl Default` 復活 → 姉妹クレート統一観点で `#[derive(Default)]` 完全復活、と二転三転した結果、最終 API は本 issue 起票前の状態に戻った。

根本原因は「Rust 単体のベストプラクティス」だけを根拠に `Default` の必要性を判断したこと。`amf-rs` / `vpl-rs` / `nvcodec-rs` の姉妹クレートが揃って `#[derive(Default)]` を持っている事実を最初に確認していれば、本 issue は起票自体不要だった。

教訓: 設定構造体・列挙型・エラー型のような「クロスクレートで一貫していると嬉しい」型については、aom 単独で derive 構成を変える issue を切る前に **姉妹クレート (amf-rs / vpl-rs / nvcodec-rs) の同名型を確認する** ことを習慣化する。
