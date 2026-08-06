# canary.py のバージョン検出・変換を [package] の version 行に限定する

- Created: 2026-08-02
- Completed: {YYYY-MM-DD}
- Branch: feature/fix-canary-version-bump
- Polished: 2026-08-06
- Reporter: @voluntas

## 目的

`canary.py` のバージョン検出・変換の正規表現が、[package] セクションの version 行以外のフィールド (例: rust-version) に誤マッチしうる潜在バグを修正する。現状は Cargo.toml のフィールド順 (version が rust-version より先) で偶然回避されているが、rust-version が version より先に並ぶと壊れる。issue 0032 (release.yml のバージョン照合) と同じく、Cargo.toml の version 読み取りは [package] セクションに限定する方針である。

## 現状

`canary.py` のバージョン検出・変換・再検証の正規表現は [package] セクションの version 行に限定されていない:

- 検出の正規表現は `rust-version = "1.93"` に誤マッチする (rust-version が version より先にある場合)。この場合、検出値が誤り、タグ名・コミットメッセージも誤る
- 変換の正規表現も同様で、`rust-version = "1.93.0"` (3 桁表記) が version より先にあると rust-version 行を書き換えてしまう
- 検出が 2 桁表記の rust-version に誤マッチした場合は、canary バージョンが非 canary 分岐に落ちて二重変換される (`2026.2.0-canary.5` が `2026.3.0-canary.0-canary.5` になる。2 桁表記のときのみ。3 桁表記のときは rust-version 行の書き換えのみ)
- 再検証 (変換後の version 抽出) も同じ正規表現のため、誤マッチ時は出力される New version も誤る

現状の Cargo.toml は version が rust-version より先に並んでいるため、この問題は発現していない。非 canary 分岐の変換ロジック自体 (minor +1 / patch 0 / canary.0 付与) は正しく動作している (`2026.2.0` → `2026.3.0-canary.0` を変換ロジックの実行で確認済み)。

## 設計方針

バージョン検出・変換・再検証のすべての正規表現を、[package] セクションの version 行に限定する。限定は行頭アンカーによるフィールド名の完全一致で行う (`\bversion\b` は rust-version 内の version にもマッチするため不可)。変換ロジック自体 (非 canary 分岐の minor +1 / patch 0 / canary.0 付与、canary 分岐の +1) は変更しない。

## 完了条件

- rust-version が version より先にある Cargo.toml でも、[package] の version を正しく検出・変換できること
- `rust-version = "1.93.0"` (3 桁表記) が version より先にあっても、version 行のみが変換されること
- 既存の変換動作が維持されること: `2026.2.0` → `2026.3.0-canary.0`、`2026.2.0-canary.5` → `2026.2.0-canary.6`

## 解決方法

- バージョン検出・変換・再検証の正規表現を、[package] セクションの version 行 (行頭アンカーによるフィールド名の完全一致) に限定する
- 変換結果の確認は既存の `--dry-run` オプションを利用する (テスト基盤は新設しない)
- 検証:
  - 一時的に Cargo.toml のフィールド順を入れ替えて (rust-version を version より先に) `--dry-run` で変換結果を確認し、元に戻す (完了条件 1)
  - あわせて rust-version の値を一時的に `"1.93.0"` (3 桁表記) に変更して同様に確認し、元に戻す (完了条件 2)
  - version 行を一時的に `2026.2.0` に変更して非 canary 分岐の変換を確認し、元に戻す (完了条件 3)
  - canary 分岐 (`2026.2.0-canary.5` → `2026.2.0-canary.6`) は現状の Cargo.toml のまま `--dry-run` で確認する (完了条件 3)
