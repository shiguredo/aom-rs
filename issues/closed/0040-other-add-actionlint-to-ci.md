# actionlint を CI に統合してワークフロー変更の構文を検証する

- Created: 2026-08-06
- Completed: 2026-08-12
- Branch: feature/update-actionlint-to-ci
- Polished: {YYYY-MM-DD}

## 目的

GitHub Actions ワークフローの変更 (YAML 構文エラー・無効な式・シェルスクリプトの問題) を CI で自動検出し、リリース運用の信頼性を確保する。ワークフローは実行テストできないため、静的な構文検証を CI に組み込むことで回帰を防ぐ。

## 現状

`.github/workflows/ci.yml` と `.github/workflows/release.yml` の変更に対する構文検証の仕組みがリポジトリにない。YAML のパース検証 (prek の check-yaml) はあるが、GitHub Actions の固有構文 (if 条件式、needs コンテキスト、gh コマンドのシェル) は検証されない。

実際に、release ワークフローの変更時に needs コンテキストの欠落 (参照先ジョブが needs に含まれていない) が起きても、CI では検出されず、実行時 (タグ push) に初めて失敗する。actionlint はこの種の問題を静的解析で検出できる。

## 設計方針

- 開発環境には actionlint が導入済み (`/opt/homebrew/bin/actionlint` 等) であることを確認済み。CI では公式の actionlint アクションまたはバイナリダウンロードで実行する
- ci.yml の fmt-clippy ジョブに actionlint の実行ステップを追加する (ワークフロー変更が PR で検出されるように)
- prek.toml の pre-commit フックにも actionlint を追加する (コミット時に検出)
- actionlint の指摘が既存のワークフローに存在する場合 (例: 既知のラベル警告) は、その扱いを判断する (無視リストへの追加 or 修正)

## 完了条件

- ci.yml の CI で actionlint が実行され、ワークフロー変更の構文エラーが PR 段階で検出されること
- prek の pre-commit で actionlint が実行されること
- 既存のワークフローで actionlint の指摘がゼロ (または明示的に許容されたもののみ) であること

## 解決方法

- 対応しない。actionlint を CI や pre-commit に導入する方針を破棄する
- 今後 actionlint には一切触れない
- ワークフローの検証は既存の YAML パース検証 (prek の check-yaml) のみに留める
