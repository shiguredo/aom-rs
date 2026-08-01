# リリースワークフローの通知とバージョン検証を整備する

- Created: 2026-08-02
- Completed: {YYYY-MM-DD}
- Branch: feature/fix-release-workflow
- Polished: {YYYY-MM-DD}
- Reporter: @voluntas

## 目的

CI / Release ワークフローの以下の問題を修正し、リリース運用の信頼性を確保する。正式リリース (2026.2.0) 前に対処すべき項目が含まれる。

## 現状

### CI 失敗時に Slack 通知が届かない

`.github/workflows/ci.yml` の `slack_notify` ジョブに `if: ${{ always() }}` がない。`needs` のジョブが 1 つでも失敗すると slack_notify 自体が skipped になり、失敗通知が一切送られない。

### Release の slack_notify に GH_TOKEN がない

`.github/workflows/release.yml` の `slack_notify` に `GH_TOKEN` env がない (ci.yml にはある)。slack-notify スキルが `gh run list` / `gh api` のため GH_TOKEN を必須と規定しており、Fixed 判定・他ジョブ失敗検出が機能しない。

### タグと Cargo.toml のバージョン一致検証がない

release.yml は任意タグ (`"*"`) で発火し、タグ名をそのまま Release の VERSION にするが、Cargo.toml の `version` との一致チェックがない。タグ `2026.2.0` を打っても publish されるのは Cargo.toml の version (canary.5 のままなら canary.5 を再 publish しようとして失敗)。Release 名と crates.io の中身が乖離する構造。

### GitHub Release が prebuilt アップロード前に public 公開される

`gh release create` が最初に実行され、prebuilt ビルドが失敗すると「アセットなしの public Release」が残る。正式リリースは `--prerelease` なしのためユーザーへの影響が大きい。

## 設計方針

- ci.yml の slack_notify に `if: ${{ always() }}` を追加する
- release.yml の slack_notify に `GH_TOKEN: ${{ github.token }}` を追加する
- github-release ジョブでタグ名と Cargo.toml の version を照合し、不一致ならジョブを失敗させる
- 正式リリースは `gh release create --draft` で作成し、全アセットアップロードと publish 成功後に公開する (canary は現行のままでよい)

## 完了条件

- CI のジョブが失敗したときに Slack 通知が届くこと
- タグと Cargo.toml の version が不一致の場合、リリースワークフローが失敗すること
- prebuilt が未完了の状態で public な正式リリースが公開されないこと

## 解決方法

上記 4 点のワークフロー修正。修正後に開発用のダミータグ (例: 次の canary) で release ワークフローが通ることを確認する。
