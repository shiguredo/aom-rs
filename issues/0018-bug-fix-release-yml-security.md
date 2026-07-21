# release.yml のコマンドインジェクションと GitHub Actions の SHA ピン留めを修正する

Created: 2026-07-21
Priority: High
Polished: 2026-07-21
Model: Qwen Code

## 概要

release.yml の `run` ブロック内で `${{ }}` 式がシェルに直接補間されており、コマンドインジェクションのリスクがある。また、2 つの GitHub Actions が SHA ピン留めではなく `@main` ブランチを参照している。

## 根拠

### コマンドインジェクション

`.github/workflows/release.yml:28-37`:

```yaml
run: |
  if ${{ contains(steps.get_version.outputs.VERSION, 'canary') }}; then
    gh release create ${{ steps.get_version.outputs.VERSION }} \
      --prerelease \
      --title "${{ steps.get_version.outputs.VERSION }}" \
      --notes "Release ${{ steps.get_version.outputs.VERSION }}"
```

`${{ steps.get_version.outputs.VERSION }}` は GitHub Actions の式評価結果がシェルスクリプトに文字列として直接埋め込まれる。`VERSION` の値は `GITHUB_REF` から生成されるタグ名そのものであり、サニタイズされていない。

同様の問題は `release.yml:102`（`gh release upload`）、`release.yml:147`（Windows ジョブ）にも存在する。

トリガーが `push: tags: "*"` であり、タグ作成にはリポジトリへの write 権限が必要。攻撃の前提条件は高いが、defense-in-depth の観点から修正すべき。

### SHA ピン留めされていないアクション

- `ci.yml:30,59,103`: `shiguredo/github-actions/.github/actions/rust-cache@main` と `slack-notify@main`
- `release.yml:175`: `shiguredo/github-actions/.github/actions/slack-notify@main`

`@main` 参照は `shiguredo/github-actions` リポジトリの改ざん時に CI パイプラインに悪意あるコードが注入される。`actions/checkout` 等は SHA ピン留め済みで基準が不統一。

### github-release ジョブの permissions 未明示

`release.yml:12-14` の `github-release` ジョブに `permissions` が明示されていない。`build-prebuilt` には `permissions: contents: write` があるが、`github-release` にはない。

### --clobber による上書き許可

`release.yml:105,150` の `gh release upload --clobber` が既存アセットの上書きを許可している。

## 修正方針

- `${{ }}` による直接補間を廃止し、`env:` で環境変数に代入した上でシェル変数 `"$VERSION"` として参照する
- `@main` 参照を SHA ピン留めに変更する
- `github-release` ジョブに `permissions: contents: write` を追加する
- `--clobber` の必要性を再検討する

## 後方互換

CI/CD の修正のみ。クレートのコード・API に変更なし。
