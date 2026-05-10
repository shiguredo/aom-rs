# ReconfigureParams の Default derive を撤廃する

Created: 2026-05-10
Completed: 2026-05-10
Model: deepseek-v4-pro

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

`src/lib.rs` の `ReconfigureParams` から `#[derive(Default)]` を撤廃した（`Debug, Clone` のみ残す）。手動 `Default` 実装も追加していない。

`tests/test_roundtrip.rs` の `test_reconfigure_empty_params_then_encode` で使用していた `ReconfigureParams::default()` を `ReconfigureParams { rc_target_bitrate: None }` に書き換えた。

`CHANGES.md` の `## develop` に `[CHANGE]` エントリを追加した。`cargo test`、`cargo clippy --all-targets --all-features -- -D warnings` がいずれも通過することを確認した。
