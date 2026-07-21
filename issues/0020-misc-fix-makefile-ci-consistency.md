# Makefile / CI / prek.toml のコマンド整合性を修正する

Created: 2026-07-21
Priority: Medium
Polished: 2026-07-21
Model: Qwen Code

## 概要

Makefile / CI / prek.toml の 3 つの実行経路でコマンドフラグが不整合。Makefile に死にターゲットが存在する。

## 根拠

### --features source-build の欠落

| コマンド | Makefile | CI | prek.toml |
|---|---|---|---|
| clippy | `cargo clippy --workspace -- -D warnings` | `cargo clippy --features source-build -- -D warnings` | `cargo clippy --workspace --all-targets --features source-build -- -D warnings` |
| test | `cargo test --workspace` | `cargo test --features source-build` | `cargo test --workspace --features source-build` |

Makefile だけ `--features source-build` が欠落。Makefile でローカル検証しても CI で失敗する可能性がある。

### --all-targets の欠落

CI の clippy には `--all-targets` がないため、テストコードと examples が clippy 検査対象外。prek.toml では検査されるが CI では見逃される。Makefile にも `--all-targets` がない。

### 死にターゲット

`Makefile:11-27` の `pbt`, `pbt-with-cover`, `fuzzing`, `fuzzing-list` は `pbt/` も `fuzz/` も存在しないため実行不能。

### .PHONY の不整合

`Makefile:1` の `.PHONY` に `pbt-cover`（正しくは `pbt-with-cover`）と `fuzz`（ターゲット定義なし）が含まれている。`pbt-with-cover` が `.PHONY` に入っていない。

### fmt-check ターゲットの欠如

Makefile の `fmt` は `cargo fmt --all`（フォーマットを適用）だが、CI / prek は `cargo fmt --all -- --check`（検査）。Makefile に検査用ターゲットがない。

## 修正方針

- Makefile の `test` / `clippy` / `cover` に `--features source-build` を追加する
- Makefile の `clippy` に `--all-targets` を追加する
- CI の clippy に `--all-targets` を追加する
- 死にターゲットを削除するか、PBT / fuzzing の実装を追加する
- `.PHONY` を実際のターゲット名と一致させる
- `fmt-check` ターゲットを追加する

## 後方互換

ビルド設定の修正のみ。クレートのコード・API に変更なし。
