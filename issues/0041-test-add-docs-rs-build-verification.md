# CI の docs-rs ジョブに DOCS_RS=1 cargo build の検証ステップを追加する

- Created: 2026-08-07
- Completed: {YYYY-MM-DD}
- Branch: feature/fix-docs-rs-ci-build-verification
- Polished: {YYYY-MM-DD}

## 目的

`.github/workflows/ci.yml` の docs-rs ジョブが `cargo doc --no-deps` のみを実行しており、rustdoc が型チェックエラーを無視するため、DOCS_RS 分岐のダミー bindings に欠落が生じても CI が検知できない。ダミー bindings の完全性を CI で常時検証できるようにする。

## 現状

- ci.yml の docs-rs ジョブは `DOCS_RS: 1` を設定して `cargo doc --no-deps` のみを実行している
- `DOCS_RS=1 cargo build` は現在 285 エラーになる (build.rs の DOCS_RS 分岐のダミー bindings の欠落) が、`cargo doc --no-deps` は rustdoc が型チェックエラーを無視するため成功する
- このため、ダミー bindings の欠落が CI をすり抜け、docs.rs 上でドキュメント生成が失敗しても検知できない

## 設計方針

- docs-rs ジョブに `DOCS_RS=1 cargo build` (型チェックを含む検証) を追加する
- `cargo doc --no-deps` はドキュメント生成の回帰確認として残す
- build.rs の DOCS_RS 分岐の修復そのもの (ダミー bindings の完全性修復) は別 issue (0037) で対応する

## 完了条件

- docs-rs ジョブで `DOCS_RS=1 cargo build` が実行されること
- ダミー bindings に欠落が生じた場合に CI が失敗すること

## 解決方法

- ci.yml の docs-rs ジョブに `DOCS_RS=1 cargo build` のステップを追加する
- 実装完了時に実際の対応内容を追記する
