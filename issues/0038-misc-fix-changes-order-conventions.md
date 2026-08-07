# CHANGES.md の種別並び順を shiguredo-changelog 規約に合わせる

- Created: 2026-08-06
- Completed: {YYYY-MM-DD}
- Branch: feature/update-changes-order-conventions
- Polished: 2026-08-07

## 目的

CHANGES.md の種別並び順が、shiguredo-changelog スキルの規約と一致していない状態を解消する。

## 現状

shiguredo-changelog スキルは「エントリは種別の順番を守って記載すること (CHANGE → ADD → UPDATE → FIX の順)」と規定している。しかし CHANGES.md は:

- `## develop` セクションが UPDATE → ADD → CHANGE → FIX の順 (規約違反)
- `### misc` セクションが UPDATE → ADD → UPDATE → FIX の順で、UPDATE が 2 ブロックに分断されている (規約違反)
- 冒頭の種別説明リスト (凡例) も UPDATE → ADD → CHANGE → FIX の順 (規約違反)

この並びは過去の対応 (closed 0019) で当時の規約に従って UPDATE → ADD → CHANGE → FIX の順に並び替えられた。その後、規約が shiguredo-changelog スキルに移管された際に順序が変わり、エントリ追加のたびに違反が継続している。

## 設計方針

- `## develop` / `### misc` セクションのエントリを CHANGE → ADD → UPDATE → FIX の順に並び替える
- 冒頭の種別説明リスト (凡例) もエントリと同じ CHANGE → ADD → UPDATE → FIX の順に並び替える
- 同種別内のエントリの相対順序は現状のまま保つ (misc の UPDATE 2 ブロックの統合時も、現状の出現順を維持する)
- エントリの内容・担当者表記は変更しない (並び順のみ)
- 並び順以外の規約違反 (エントリの「〜する」形式等) は本 issue のスコープ外
- リリース済みセクション (## 2026.1.0 等) は履歴のため対象外
- shiguredo-changelog スキルの規約が正であるとみなす (スキルが最新の規約ソース)

## 完了条件

- 冒頭の種別説明リスト・`## develop` ・`### misc` のエントリが CHANGE → ADD → UPDATE → FIX の順で並んでいること
- 同種別内のエントリの相対順序が現状のままであること
- エントリの内容・担当者表記が変更されていないこと

## 解決方法

- CHANGES.md の冒頭の種別説明リスト・`## develop` / `### misc` セクションのエントリを並び替える
- 検証: 並び替え後に、冒頭の種別説明リストと各セクションの種別順、および同種別内のエントリの相対順序を目視確認する。内容の差分がないことは、並び替え前 (git show origin/develop:CHANGES.md で取得) と並び替え後のエントリを、セクションごと (develop / misc に分けて)「- [種別]」で始まる行と担当者行のペア単位で抽出して順序を無視して突き合わせることで確認する (エントリ行と担当者行はペアで移動する。セクション跨ぎの移動を検出するためセクションごとに比較する。冒頭の種別説明リストはこの抽出の対象外とし、内容不変は目視で確認する。git diff は行移動を削除 + 追加として表示するため、単純な diff では内容不変を確認できない)
