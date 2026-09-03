# CI の docs-rs ジョブに DOCS_RS=1 cargo build の検証ステップを追加する

- Created: 2026-08-07
- Completed: 2026-09-03
- Branch: feature/add-docs-rs-ci-build-verification
- Polished: 2026-08-12

## 目的

`.github/workflows/ci.yml` の docs-rs ジョブが `cargo doc --no-deps` のみを実行しており、rustdoc が型チェックエラーを無視するため、DOCS_RS 分岐のダミー bindings に欠落が生じても CI が検知できない。ダミー bindings の完全性を CI で常時検証できるようにする。

## 現状

- ci.yml の docs-rs ジョブは `DOCS_RS: 1` を設定して `cargo doc --no-deps` のみを実行している (型チェックされないため、ダミー bindings に欠落が生じても成功する)
- ダミー bindings の完全性は closed issue 0037 で修復済みであり、`DOCS_RS=1 cargo build` は現状成功する
- しかし CI には build 検証ステップが無いため、将来ダミー bindings に欠落が生じた場合に CI をすり抜け、docs.rs 上で欠落したドキュメントが生成されても検知できない (0037 の修復前は `cargo doc --no-deps` が成功し CI をすり抜けていた実測がある)

## 設計方針

- docs-rs ジョブに `DOCS_RS=1 cargo build` (型チェックを含む検証) を `cargo doc --no-deps` の前に追加する
- `cargo doc --no-deps` はドキュメント生成の回帰確認として残す
- `cargo build` は `cargo check` より強い検証 (コンパイル + コード生成) であり、欠落が生じた場合に E0425 等で確実に失敗するため採用する。DOCS_RS 分岐では libaom 自体のビルドは発生しないため実行時間の増加は軽微である (docs-rs ジョブの timeout-minutes: 15 に収まることを CI 実測で確認する)
- ジョブの `env: DOCS_RS: 1` は既に設定済みのため、追加ステップは `cargo build` の記述のみでよい
- build.rs の DOCS_RS 分岐の修復 (ダミー bindings の完全性修復) は closed issue 0037 で対応済みであり、本 issue のスコープ外

## 完了条件

- docs-rs ジョブで `DOCS_RS=1 cargo build` が実行されること (CI ログで cargo build ステップの実行を確認する)
- ダミー bindings に欠落が生じた場合に CI が失敗すること (検証手順: `build/dummy_bindings.rs` から src/lib.rs が参照するシンボルを 1 つ削除して `DOCS_RS=1 cargo build` が失敗すること、復元して成功することをローカルで確認する)
- feature ブランチを push した後に CI の docs-rs ジョブが成功すること
- CI 設定のみの変更のため、CHANGES.md には misc セクションに [ADD] エントリを追記する

## 解決方法

- `.github/workflows/ci.yml` の docs-rs ジョブに `cargo build` ステップを `cargo doc --no-deps` の前に追加した (ジョブの `env: DOCS_RS: 1` により型チェック付きビルドとして実行される)
- 検証: `build/dummy_bindings.rs` から `aom_codec_av1_cx` を削除して `DOCS_RS=1 cargo build` が E0425 で失敗すること、復元して成功することをローカルで確認した
