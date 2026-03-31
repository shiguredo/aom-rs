# 変更履歴

- UPDATE
  - 後方互換がある変更
- ADD
  - 後方互換がある追加
- CHANGE
  - 後方互換のない変更
- FIX
  - バグ修正

## develop

- [CHANGE] `EncodedFrame::data()` の戻り値を `&[u8]` から `Result<&[u8], Error>` に変更する
  - @voluntas
