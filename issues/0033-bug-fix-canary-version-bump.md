# canary.py の非 canary バージョン変換を修正する

- Created: 2026-08-02
- Completed: {YYYY-MM-DD}
- Branch: feature/fix-canary-version-bump
- Polished: {YYYY-MM-DD}
- Reporter: @voluntas

## 目的

`canary.py` のバージョン変換ロジックが不正なバージョンを生成し、正式リリース後の develop バンプに使用できない状態を修正する。

## 現状

`canary.py` の非 canary 分岐は `(version)(major).(minor).(patch)` 形式の patch に 1 を足して `.0-canary.0` を付与する。そのため `2026.2.0` を渡すと `2026.2.1.0-canary.0` という不正なバージョン (4 つの数字) を生成する。正しくは `2026.3.0-canary.0` (minor を +1、patch を 0 に) である。

また、バージョン検出の正規表現が `rust-version = "1.93"` のようなフィールドにもマッチしうる。現状は Cargo.toml のフィールド順 (version が rust-version より先) で偶然回避されているだけで、フィールド順の変更で壊れる。

## 設計方針

- 非 canary 分岐の変換を「minor +1 / patch 0 / canary.0 付与」に修正する
- バージョン検出の正規表現を `[package]` セクションの `version` 行に限定する (フィールド名の完全一致)

## 完了条件

- `2026.2.0` を入力として `2026.3.0-canary.0` に変換されること
- `2026.2.0-canary.5` を入力として `2026.2.0-canary.6` に変換されること (既存動作の維持)
- 変換結果の確認手段 (テストまたは dry-run) が存在すること

## 解決方法

- 変換ロジックと正規表現の修正
- 変換結果を検証する手段の追加 (ユニットテスト or dry-run での出力確認)
