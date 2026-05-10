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

- [ADD] `Encoder::reconfigure(ReconfigureParams)` を追加してランタイムでエンコード設定を変更できるようにする
  - @voluntas
- [CHANGE] `ReconfigureParams` から `g_w` / `g_h` / `g_timebase` フィールドを削除する
  - @voluntas
- [FIX] `Encoder::reconfigure()` の libaom 呼び出し失敗時に内部 `cfg` が変更前のまま保持されるよう修正する
  - @voluntas

### misc


## 2026.1.0

**リリース日**: 2026-03-31
