# CI / Release ワークフローを修正する

- Created: 2026-08-02
- Completed: 2026-08-06
- Branch: feature/fix-release-workflow
- Polished: 2026-08-06
- Reporter: @voluntas

## 目的

CI / Release ワークフローの以下の問題を修正し、リリース運用の信頼性を確保する。

## 現状

### CI 失敗時に Slack 通知が届かない

`.github/workflows/ci.yml` の `slack_notify` ジョブに `if: ${{ always() }}` がない。`needs` のジョブが 1 つでも失敗すると slack_notify 自体が skipped になり、失敗通知が一切送られない。

### Release の slack_notify が ci.yml と非対称

`.github/workflows/release.yml` の `slack_notify` は `if: ${{ always() }}` を持つが、ci.yml と異なり `GH_TOKEN` env を渡していない。slack-notify アクションは composite action 内部で `GH_TOKEN` を設定するため実害はないが、slack-notify スキルが呼び出し側での env 渡しを推奨しており、ci.yml との一貫性がない。

### タグと Cargo.toml のバージョン一致検証がない

release.yml は任意タグ (`"*"`) で発火し、タグ名をそのまま Release の VERSION にするが、Cargo.toml の version との一致チェックがない。タグ `2026.2.0` を打っても publish されるのは Cargo.toml の version (canary.5 のままなら、publish 済みの canary.5 の再 publish に失敗するか、乖離した version が publish される)。Release 名と crates.io の中身が乖離する構造。

### GitHub Release が prebuilt アップロード前に public 公開される

`gh release create` が最初に実行され、prebuilt ビルドが失敗すると「アセットなしの public Release」が残る。正式リリースは `--prerelease` なしのためユーザーへの影響が大きい。

## 設計方針

- ci.yml の `slack_notify` に `if: ${{ always() }}` を追加する (paths-ignore により .md のみの変更ではワークフロー自体が発火しないため、全 skipped 時の制御は不要)
- release.yml の `slack_notify` に `GH_TOKEN: ${{ github.token }}` を追加する (スキルの推奨と ci.yml との一貫性のため)
- github-release ジョブで、`gh release create` の前にタグ名と Cargo.toml [package] の version を照合し、不一致ならジョブを失敗させる (Cargo.toml の version 読み取りは [package] セクションに限定する。package.metadata.external-dependencies.aom にも version があるため)
- 正式リリースは `gh release create --draft` で作成し、publish ジョブの `cargo publish` 成功後に `gh release edit --draft=false` で公開する (publish ジョブの permissions に `contents: write` を追加する。canary は現行のままでよい)

## 完了条件

- CI のジョブが失敗したときに Slack 通知が届くこと
- release.yml の `slack_notify` に ci.yml と同様の `GH_TOKEN` env が追加されていること (コードレビューで確認)
- タグと Cargo.toml の version が不一致の場合、GitHub Release が作成される前にリリースワークフローが失敗すること
- prebuilt が未完了の状態で public な正式リリースが公開されないこと

## 解決方法

- `.github/workflows/ci.yml` の `slack_notify` に `if: ${{ always() }}` を追加した (ジョブ失敗時に Slack 通知が送られるように)
- `.github/workflows/release.yml` の `slack_notify` に `GH_TOKEN: ${{ github.token }}` env を追加した (slack-notify スキルの推奨と ci.yml との一貫性)。あわせて `timeout-minutes` を ci.yml と同じ 5 分に統一した
- github-release ジョブにタグ名と Cargo.toml [package] の version の照合ステップを追加した (`gh release create` の前。不一致ならジョブが失敗する)。version 読み取りは sed で [package] セクションに限定している (package.metadata.external-dependencies.aom の version を誤読しない)
- github-release ジョブに既存 draft の削除ステップを追加した (前回失敗で残った同名 draft があると同一タグの再実行が失敗するため。isDraft チェックで public release は削除しない)
- 正式リリースを `gh release create --draft` で作成するように変更し、cargo publish 成功後に実行される `undraft-release` ジョブ (needs: publish + github-release、正式リリースのみ) が `gh release edit --draft=false` で公開するようにした。canary は従来どおり prerelease として作成時に公開される
- CHANGES.md の misc セクションに記録した
