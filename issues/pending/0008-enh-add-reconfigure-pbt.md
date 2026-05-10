# reconfigure の PBT テストを追加する

Created: 2026-05-10
Model: deepseek-v4-pro

## Pending 理由

本 issue は AGENTS.md (CLAUDE.md) に存在した「PBT(Property-Based Testing) や Fuzzing で必ずテストを行うこと」「unittest は pbt で実現できないものだけを書く」というルールを根拠に作成された。しかしその後 AGENTS.md からこれらのルールおよび `## テストについて` / `## Rust` 節が削除されている (2026-05-10)。前提ルールが失われたため、本 issue の「PBT を新規導入する」という結論は再検討が必要であり、いったん `pending` に置く。再開時は (a) PBT を導入するか (b) 既存の単体テスト・回帰テストで十分とするかを判断したうえで設計を更新する。

## 概要

CLAUDE.md に「PBT(Property-Based Testing) や Fuzzing で必ずテストを行う」「unittest は pbt で実現できないものだけを書く」と明記されているが、reconfigure の PBT テストが 0 件である。プロジェクト全体でも `pbt/` ディレクトリが存在しない。

## 根拠

reconfigure の以下の性質は PBT に適している:

1. **ラウンドトリップ不変性**: 任意の `ReconfigureParams` に対して、reconfigure 後にエンコード・デコードが正常に完走すること
2. **冪等性**: 同じパラメータで複数回 reconfigure を呼んでも結果が変わらないこと
3. **フレーム数不変性**: reconfigure がエンコード済みフレーム数に影響しないこと

## 修正方針

1. `pbt/` ディレクトリを作成する
2. proptest を用いた PBT テストを追加する
3. 最低限以下のテストを含める:
   - `prop_reconfigure_preserves_roundtrip`: ランダムな `ReconfigureParams` で reconfigure してもエンコード・デコードが完走する
   - `prop_reconfigure_is_idempotent`: 同じパラメータでの複数回呼び出しが同じ結果を返す
   - `prop_reconfigure_preserves_frame_count`: reconfigure 前後でデコードフレーム数が変わらない

### proptest 戦略

```rust
proptest! {
    #[test]
    fn prop_reconfigure_preserves_roundtrip(
        target_bitrate in 1u32..10000,
        num in 1i32..1000,
        den in 1i32..1000,
        reconfigure_at in 1usize..20,
    ) {
        // ...
    }
}
```

## 参考

- レビュー指摘: `feature/encoder-reconfigure` ブランチの `/review-diff-code` 結果より（重要指摘 4.1, テスト指摘 7）
- CLAUDE.md: 「PBT(Property-Based Testing) や Fuzzing で必ずテストを行うこと」
