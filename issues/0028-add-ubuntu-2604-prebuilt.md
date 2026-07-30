# ubuntu-26.04 / ubuntu-26.04-arm の prebuilt バイナリをサポートする

- Created: 2026-07-31
- Completed: 2026-07-31
- Branch: feature/add-ubuntu-2604-prebuilt
- Polished: 2026-07-31
- Model: Qwen Code qwen3.8-max-preview

## 目的

CI で ubuntu-26.04 / ubuntu-26.04-arm をテストしているにもかかわらず、prebuilt パスと release ワークフローが未対応のため、ubuntu-26.04 ユーザーが `source-build` feature なしでビルドできない問題を解消する。

## 現状

- `.github/workflows/ci.yml` の test matrix に `ubuntu-26.04` と `ubuntu-26.04-arm` が含まれている
- `build.rs` の `detect_linux_distro` 関数は `"22.04" | "24.04"` 以外で panic する
- `.github/workflows/release.yml` の `build-prebuilt` matrix に ubuntu-26.04 系がない
- `README.md` の動作要件に ubuntu-26.04 が記載されていない

つまり CI でテストしている OS にもかかわらず、エンドユーザーがデフォルトの `cargo build`（prebuilt パス）を実行すると panic する（CI 自体は `--features source-build` で動作するためこの panic に遭遇しない）。

## 設計方針

- `build.rs` の `detect_linux_distro` に `"26.04"` を追加する
- `.github/workflows/release.yml` の `build-prebuilt` matrix に `ubuntu-26.04_x86_64` と `ubuntu-26.04_arm64` を追加する
- `README.md` の動作要件に ubuntu-26.04 x86_64 / arm64 を追加する

## 完了条件

- ubuntu-26.04 上で `cargo build`（prebuilt パス）が成功する
- release ワークフローで ubuntu-26.04 系の prebuilt アーカイブが生成・アップロードされる
- README の動作要件に ubuntu-26.04 が記載される
- 既存の全テストが通過する

## 解決方法

- `build.rs` の `detect_linux_distro` 関数の match arm に `"26.04"` を追加した
- `.github/workflows/release.yml` の `build-prebuilt.strategy.matrix.include` に `ubuntu-26.04_x86_64`（os: ubuntu-26.04）と `ubuntu-26.04_arm64`（os: ubuntu-26.04-arm）を追加した
- `README.md` の動作要件リストに `Ubuntu 26.04 x86_64` と `Ubuntu 26.04 arm64` を追加した
