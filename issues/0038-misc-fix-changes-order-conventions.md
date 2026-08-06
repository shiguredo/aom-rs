# CHANGES.md の種別並び順を shiguredo-changelog 規約に合わせる

- Created: 2026-08-06
- Completed: {YYYY-MM-DD}
- Branch: feature/refactor-changes-order-conventions
- Polished: {YYYY-MM-DD}

## 目的

CHANGES.md の `## develop` セクションの種別並び順が、shiguredo-changelog スキルの規約と一致していない状態を解消する。

## 現状

shiguredo-changelog スキルは「エントリは種別の順番を守って記載すること (CHANGE → ADD → UPDATE → FIX の順)」と規定している。しかし CHANGES.md の `## develop` セクションは UPDATE → ADD → CHANGE → FIX の順で並んでおり、規約に違反している。

この並びは closed 0019 の対応で「UPDATE → ADD → CHANGE → FIX の順」に並び替えられたもので、当時の規約 (AGENTS.md の記述) に従った結果である。規約が shiguredo-changelog スキルに移管された際に順序が変わり、実装が追従していない可能性が高い。エントリ追加のたびに違反が継続・拡大している。

## 設計方針

- `## develop` セクションのエントリを CHANGE → ADD → UPDATE → FIX の順に並び替える
- `### misc` セクションも同様に規約 (CHANGE → ADD → UPDATE → FIX) に合わせる
- エントリの内容・担当者表記は変更しない (並び順のみ)
- shiguredo-changelog スキルの規約が正であるとみなす (スキルが最新の規約ソース)

## 完了条件

- `## develop` と `### misc` のエントリが CHANGE → ADD → UPDATE → FIX の順で並んでいること
- エントリの内容・担当者表記が変更されていないこと

## 解決方法

- CHANGES.md の `## develop` / `### misc` セクションのエントリを並び替える
- 検証: 並び替え後に種別順を目視確認し、内容の差分がないことを `git diff` で確認する
