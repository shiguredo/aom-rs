# DOCS_RS ダミー bindings の完全性を検証・修復する

- Created: 2026-08-06
- Completed: {YYYY-MM-DD}
- Branch: feature/fix-docs-rs-dummy-bindings
- Polished: {YYYY-MM-DD}

## 目的

build.rs の DOCS_RS 分岐で生成されるダミー bindings が、src/lib.rs が参照するシンボルの大部分を欠落したままになっており、`DOCS_RS=1 cargo build` が失敗する状態を解消する。docs.rs 上のドキュメント生成が正しく行われることを保証する。

## 現状

build.rs の DOCS_RS 分岐のダミー bindings は少数の型・定数のみを定義している (struct 7 個 + type 2 個 + const 数個)。src/lib.rs は 100 個以上の `sys::` シンボルを参照しており、`DOCS_RS=1 cargo build` は `aom_enc_pass_AOM_RC_ONE_PASS` 等の欠落定数で数百件のエラーになる (実証済み)。

`DOCS_RS=1 cargo doc --no-deps` は rustdoc が型チェックエラーを無視するため成功してしまい、CI の docs-rs ジョブ (.github/workflows/ci.yml の docs-rs) は実質的に検証になっていない。過去の対応 (closed 0022) では `rerun-if-env-changed=DOCS_RS` の追加のみ行われ、完全性の検証・修復は未完了のまま closed されている。

0029 の対応で `AOM_CODEC_USE_HIGHBITDEPTH` 定数と `aom_codec_flags_t` 型を追加したが、これは必要最小限の追加であり、ダミー全体の完全性は依然として不足している。

## 設計方針

- src/lib.rs が参照する全 `sys::` シンボルを抽出し、ダミー bindings が過不足なくカバーしていることを検証する
- 欠落しているシンボルをダミー bindings に追加する
- 検証は `cargo clean && DOCS_RS=1 cargo build` で行う (`cargo doc --no-deps` は型チェックエラーを無視するため検証として不十分)
- ダミーは docs.rs 専用であり、実 libaom の値と一致させる必要はない (型・定数の存在のみが必要) が、既知の実値は正確に記載する

## 完了条件

- `cargo clean && DOCS_RS=1 cargo build` が成功すること
- `DOCS_RS=1 cargo doc --no-deps` が成功すること
- 通常ビルド (prebuilt / source-build) が通ること (回帰なし)

## 解決方法

- src/lib.rs の `sys::` 参照を網羅的に抽出し、欠落シンボルをダミー bindings に追加する
- `cargo clean && DOCS_RS=1 cargo build` で検証する
