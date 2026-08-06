# テストヘルパーの 8-bit / 16-bit 重複を共通化する

- Created: 2026-08-06
- Completed: {YYYY-MM-DD}
- Branch: feature/refactor-test-helpers-dedup
- Polished: {YYYY-MM-DD}

## 目的

`tests/helpers/mod.rs` の 8-bit 系と 16-bit 系のヘルパーで重複している実装を共通化し、メンテナンス性を向上させる。

## 現状

`tests/helpers/mod.rs` に 8-bit 系と 16-bit 系のヘルパーが並存しており、以下の重複がある:

- `psnr_y` と `psnr_y_16bit`: MSE 計算ループが完全に同一で、最大値 (255 vs `(1 << bit_depth) - 1`) のみが異なる
- `generate_colorbar_i420` と `generate_colorbar_16bit`: SMPTE カラーバーの bars 配列 (7 要素) と BT.601 RGB→YUV 変換式が完全に重複 (16-bit 版はスケール係数とサブサンプリングが追加)
- `encode_frames` と `encode_frames_16bit`: `finish()` 後のドレインループが、同一ファイル内に既存の `drain_after_finish` ヘルパーと重複 (16-bit 版は `drain_after_finish` を利用済みだが、8-bit 版は自前実装のまま)

## 設計方針

- 挙動は一切変えない (テストヘルパーの内部構造の整理のみ)
- bars 配列はモジュールレベル const に集約する
- PSNR は最大値パラメータを取る共通関数に統合する (8-bit / 16-bit 両対応)
- `encode_frames` の finish 後ドレインは `drain_after_finish` を利用する形に統一する
- 既存テストの挙動が変わらないことを全テスト通過で担保する

## 完了条件

- `psnr_y` と `psnr_y_16bit` の MSE 計算ループが共通化されていること
- bars 配列の定義が 1 箇所に集約されていること
- `encode_frames` が `drain_after_finish` を利用していること
- 全テストが通ること (回帰なし)

## 解決方法

- `tests/helpers/mod.rs` をリファクタリングする (const 化・共通関数化・`drain_after_finish` の利用)
- 検証: リファクタリング後に全テストを実行し、既存テストの結果が変わらないことを確認する
