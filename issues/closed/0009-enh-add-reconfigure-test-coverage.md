# reconfigure のテストカバレッジを拡充する

Created: 2026-05-10
Completed: 2026-05-10
Model: deepseek-v4-pro

## 概要

reconfigure の既存テストは成功パスのみで、エラーパス・境界値・複数回連続呼び出し・VBR モード等が未カバーである。さらに `test_reconfigure_immediately_after_new` は reconfigure 後のエンコード動作を検証していない。

## 根拠

### 不足しているテストケース

1. **エラーパス**:
   - `encode()` 後に `next_frame()` を呼ぶ前に `reconfigure()` を呼ぶとエラーになること（iter 非 NULL ガード）
   - `finish()` 後に `next_frame()` を呼ぶ前に `reconfigure()` を呼ぶとエラーになること

2. **エンコード検証**:
   - `test_reconfigure_immediately_after_new` は `reconfigure()` が `Ok` を返すことだけを確認し、その後のエンコードを検証していない。`rc_target_bitrate: Some(500)` や `g_timebase: Some(AomRational { num: 1, den: 60 })` が実際に反映されているか不明

3. **複数回連続呼び出し**:
   - ビットレートを 1000→2000→500→1500 のように連続変更するケース

4. **レート制御モード**:
   - 全テストが CBR のみ。VBR での reconfigure もテストすべき

5. **finish 後の reconfigure**:
   - `finish()` 後の ctx に対して `aom_codec_enc_config_set()` が呼ばれた場合の挙動検証

## 修正方針

`tests/test_roundtrip.rs` に以下を追加する:

1. `test_reconfigure_while_iter_active` — encode 後 next_frame 未完了中に呼ぶとエラー
2. `test_reconfigure_after_finish_while_iter_active` — finish 後 next_frame 未完了中に呼ぶとエラー
3. `test_reconfigure_immediately_after_new` を拡張し、reconfigure 後に数フレームエンコード・デコードする
4. `test_reconfigure_target_bitrate_multi_switch` — 複数回の連続切り替え
5. `test_reconfigure_target_bitrate_vbr` — VBR モードでの midstream reconfigure
6. `test_reconfigure_empty_params_then_encode` — 空パラメータでの reconfigure 後にエンコード

## 参考

- レビュー指摘: `feature/encoder-reconfigure` ブランチの `/review-diff-code` 結果より（テスト指摘 1, 2, 6, 8, 9, 10, 11）

## 解決方法

`tests/test_roundtrip.rs` の reconfigure テスト群を以下の構成に整理した。

- `test_reconfigure_target_bitrate_midstream`: 既存。midstream でのビットレート切り替え (CBR)
- `test_reconfigure_immediately_after_new`: 既存を拡張し、reconfigure 後に複数フレームの encode / decode まで検証する形にした
- `test_reconfigure_empty_params_then_encode`: `test_reconfigure_empty_params_is_noop` を置き換え。空パラメータでの reconfigure 後に encode / decode が完走することを検証
- `test_reconfigure_state_unchanged_on_failure`: 0005 で導入済み
- `test_reconfigure_while_iter_active`: encode 後 next_frame 未完了で reconfigure すると `Encoder::reconfigure` を含むエラーになることを検証
- `test_reconfigure_after_finish_while_iter_active`: GoodQuality VBR + `g_lag_in_frames = 4` でフレームを蓄積し、`finish()` 後 next_frame 未完了で reconfigure するとエラーになることを検証 (realtime モードでは `finish()` 後にフレームが残らないため)
- `test_reconfigure_target_bitrate_multi_switch`: 1000→2000→500→1500 の連続切り替えで encode / decode が完走することを検証
- `test_reconfigure_target_bitrate_vbr`: VBR モードでの midstream reconfigure を検証

`cargo test`、`cargo clippy --all-targets --all-features -- -D warnings` がいずれも通過することを確認した。
