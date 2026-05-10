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

- [ADD] `Encoder::reconfigure(ReconfigureParams)` を追加する
  - @voluntas
- [CHANGE] `ReconfigureParams` から `g_w` / `g_h` / `g_timebase` フィールドを削除する
  - @voluntas
- [CHANGE] `ReconfigureParams` から `Default` derive を撤廃する
  - @voluntas
- [FIX] `Encoder::reconfigure()` の libaom 呼び出し失敗時に内部 `cfg` が変更前のまま保持されるよう修正する
  - @voluntas

### misc

- `Encoder::reconfigure()` の doc コメントに libwebrtc 方式の運用方針 (エンコーダーを破棄せず本メソッドで更新する / timebase 固定運用 / 将来 SVC 拡張時の順序制約) を追記する
  - @voluntas
- `examples/midstream_reconfigure.rs` を追加し、30fps エンコード途中でビットレートを切り替える典型パターンを示す
  - @voluntas


## 2026.1.0

**リリース日**: 2026-03-31
