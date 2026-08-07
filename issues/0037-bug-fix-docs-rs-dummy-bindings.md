# DOCS_RS ダミー bindings の完全性を検証・修復する

- Created: 2026-08-06
- Completed: {YYYY-MM-DD}
- Branch: feature/fix-docs-rs-dummy-bindings
- Polished: 2026-08-07

## 目的

build.rs の DOCS_RS 分岐で生成されるダミー bindings が、src/lib.rs が参照するシンボルの大部分を欠落したままになっており、`DOCS_RS=1 cargo build` が失敗する状態を解消する。docs.rs 上のドキュメント生成が正しく行われることを保証する。

## 現状

build.rs の DOCS_RS 分岐のダミー bindings は少数の型・定数のみを定義している (struct 7 個 + type 2 個 + const 数個)。src/lib.rs は 119 個の `sys::` シンボル (うち 2 個は metadata.rs 由来の `BUILD_METADATA_*`) を参照しており、`DOCS_RS=1 cargo build` は `aom_enc_pass_AOM_RC_ONE_PASS` 等の欠落シンボルで 285 エラー (環境により前後する) になることを実測で確認している。エラーの内訳は欠落シンボル (E0425) のほか、型・定数の欠落 (E0531 / E0422)・struct のフィールド不存在 (E0609)・関数シグネチャ不一致 (E0599 / E0308)・トレイト実装不足 (E0277) を含み、「欠落シンボルの追加」だけで完結しない。

`DOCS_RS=1 cargo doc --no-deps` は rustdoc が型チェックエラーを無視するため成功してしまい、CI の docs-rs ジョブ (.github/workflows/ci.yml の docs-rs) は実質的に検証になっていない。過去の対応 (closed 0022) では `rerun-if-env-changed=DOCS_RS` の追加のみ行われ、完全性の検証・修復は未完了のまま closed されている。

## 設計方針

- src/lib.rs が参照する全 `sys::` シンボル (metadata.rs 由来の `BUILD_METADATA_*` は対象外) を抽出し、ダミー bindings が過不足なくカバーしていることを検証する。`aom_codec_cx_pkt` のように `sys::` 直接参照には現れないが、`aom_codec_get_cx_data` の戻り値型として暗黙に必要な型も、抽出した定義の依存関係から漏らさない
- ダミーは「型・定数の存在のみ」では足りず、lib.rs が実際に使用する形と互換な定義が必要 (struct リテラルによる初期化・フィールドアクセス・関数シグネチャ・`Debug` 等のトレイト実装)。source-build で生成された bindings.rs を元に、lib.rs が参照するシンボルとその依存定義のみをコピーして構成する (コピー範囲の判定は参照シンボル一覧との突き合わせで行う)
- ダミー bindings の出力方法は現行の `concat!` 文字列リテラルに限らず、コピーする定義の量に応じて、ダミー定義を build.rs 外のファイルに置いて `include_str!` で読み込む方式等に変更してよい
- ダミーの定数値は docs.rs のドキュメント表示専用であり、libaom v3.14.1 のヘッダから確認できる既知の実値は正確に記載する
- 検証は `cargo clean && DOCS_RS=1 cargo build` で行う
- CI の docs-rs ジョブの変更 (build 検証ステップの追加) は本 issue のスコープ外とし、別 issue (0041) で対応する

## 完了条件

- `cargo clean && DOCS_RS=1 cargo build` が成功すること (主検証)
- `DOCS_RS=1 cargo doc --no-deps` が成功すること (回帰確認。rustdoc は型チェックエラーを無視するため、主検証は `cargo build` の成功である)
- ダミー bindings に lib.rs が参照しない不要な定義が含まれていないこと (参照シンボル一覧との突き合わせで確認する)
- 既知の定数の実値が libaom v3.14.1 のヘッダの値と一致していること
- 通常ビルド (prebuilt / source-build) が通ること (回帰なし)

## 解決方法

- source-build で生成された bindings.rs を出力先から取得し、lib.rs が使用する `sys::` シンボルとその依存型・定数の定義をダミー bindings にコピーする
- `cargo clean && DOCS_RS=1 cargo build` でビルドが通ることと、`DOCS_RS=1 cargo doc --no-deps` でドキュメントが生成されることを確認する
- 通常ビルドは `cargo build` (prebuilt) と `cargo build --features source-build` の両方で通ることを確認する (回帰なし)
