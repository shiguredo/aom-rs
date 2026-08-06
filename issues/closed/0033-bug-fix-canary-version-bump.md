# canary.py のバージョン検出・変換を [package] の version 行に限定する

- Created: 2026-08-02
- Completed: 2026-08-06
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

- バージョン検出・変換・再検証のすべての正規表現を、行頭アンカーによるフィールド名の完全一致に限定した (rust-version 等への誤マッチを防止)
- 検出・再検証は `VERSION_LINE_RE` 定数 (`^\s*version\s*=\s*"([\w.-]+)"` + MULTILINE) に一元化した
- [package] セクションの範囲は行頭アンカー (`^\s*\[package\]`) で検出し、次の任意の [ セクションで境界にした (release.yml のバージョン照合と同じ定義。package.metadata の version 行を検出対象外にした)
- 変換の正規表現も行頭アンカー + MULTILINE にし、インデントを保持する形にした
- 変換ロジック自体 (minor +1 / canary +1) は変更していない
- 検証: 一時 Cargo.toml でフィールド順の入れ替え (rust-version を version より先に) と rust-version の 3 桁表記化を行い、`--dry-run` で変換結果を確認した (完了条件 1・2)。非 canary 分岐 (2026.2.0 → 2026.3.0-canary.0) と canary 分岐 (2026.2.0-canary.5 → 2026.2.0-canary.6) も `--dry-run` で確認した (完了条件 3)。修正前コードで rust-version への誤マッチ (1.93 検出) が再現することも確認した
