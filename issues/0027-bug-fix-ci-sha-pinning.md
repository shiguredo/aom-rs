# CI / release ワークフローの composite action を SHA ピン留めする

- Created: 2026-07-31
- Completed: {YYYY-MM-DD}
- Branch: feature/fix-ci-sha-pinning
- Polished: {YYYY-MM-DD}
- Model: Qwen Code qwen3.8-max-preview

## 目的

`shiguredo/github-actions` の composite action 参照を `@main` から SHA ピン留め + バージョンコメント形式に変更して、サプライチェーン攻撃の侵入経路を塞ぐ。

## 現状

`actions/checkout` は SHA ピン留めされている:

```yaml
uses: actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd # v6.0.2
```

一方、自社 composite action は mutable ref の `@main` で参照している:

- `.github/workflows/ci.yml` の `shiguredo/github-actions/.github/actions/rust-cache@main`（2 箇所）
- `.github/workflows/ci.yml` の `shiguredo/github-actions/.github/actions/slack-notify@main`（1 箇所）
- `.github/workflows/release.yml` の `shiguredo/github-actions/.github/actions/slack-notify@main`（1 箇所）

`@main` は mutable ref であり、`shiguredo/github-actions` リポジトリが侵害された場合に CI/CD パイプライン全体が汚染される。

## 設計方針

- `shiguredo/github-actions` リポジトリの `main` ブランチの最新コミットハッシュを取得する
- `uses: owner/repo@<コミットハッシュ> # <バージョンまたはブランチ名>` 形式に統一する
- 今後 `shiguredo/github-actions` 側がリリースタグを付与した場合はタグ + SHA 形式に移行する

## 完了条件

- 全ワークフローファイルの composite action 参照が SHA ピン留めされる
- CI が正常に通過する

## 解決方法

- `gh api repos/shiguredo/github-actions/commits/main --jq .sha` で最新コミットハッシュを取得する
- `.github/workflows/ci.yml` と `.github/workflows/release.yml` の `@main` を `@<SHA> # main` に置換する
