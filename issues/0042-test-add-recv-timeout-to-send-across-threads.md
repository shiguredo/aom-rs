# スレッド間移動テストに mpsc 受信タイムアウトを導入する

- Created: 2026-08-07
- Completed: {YYYY-MM-DD}
- Branch: feature/fix-recv-timeout-in-send-across-threads
- Polished: {YYYY-MM-DD}

## 目的

`tests/send_across_threads.rs` のスレッド間移動テストが、libaom 内部のデッドロック時に無限にハングし、`cargo test` 全体 (CI を含む) が止まるリスクを解消する。

## 現状

- スレッド間移動テストの `test_encoder_send_across_threads` / `test_decoder_send_across_threads` は、子スレッドから mpsc で送信される値を `rx.recv()` で受信している
- `rx.recv()` は子スレッドが値を送信するまで無制限にブロックする
- 検証対象は「移動後の libaom ワーカーが正常動作するか」であり、万一 libaom 内部でデッドロックが発生すると子スレッドがハングし、`rx.recv()` が無限ブロックしてテスト全体がハングする
- `cargo test` には per-test タイムアウトが無いため、ハングすると CI 全体が停止する

## 設計方針

- cargo nextest は採用しない (プロジェクト方針)
- `cargo test` のままで対応し、`rx.recv_timeout` を導入して受信にタイムアウトを設ける
- タイムアウト時はテストを失敗させる (ハングではなくテスト失敗として報告する)
- 子スレッドのハングはテストプロセス終了時に破棄されるため、これまでの `join` による結果検査は維持する

## 完了条件

- スレッド間移動テストの受信がタイムアウト付きになり、子スレッドがハングしてもテストが失敗として終了すること
- 既存テストが通ること (回帰なし)

## 解決方法

- `tests/send_across_threads.rs` の `rx.recv()` を `rx.recv_timeout` に変更し、タイムアウト時は日本語メッセージ付きで失敗させる
- 実装完了時に実際の対応内容を追記する
