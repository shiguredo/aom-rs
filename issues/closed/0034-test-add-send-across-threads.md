# Encoder / Decoder のスレッド間移動検証テストを追加する

- Created: 2026-08-02
- Completed: 2026-08-07
- Branch: feature/add-send-across-threads-test
- Polished: 2026-08-07
- Reporter: @voluntas

## 目的

`unsafe impl Send` の安全性根拠がコメントのみで、実際にスレッド間移動して動作することを実証するテストが存在しない。unsafe コードの検証としては不十分である。

## 現状

- `Encoder` (unsafe impl Send) と `Decoder` (unsafe impl Send) を宣言している
- 安全性の根拠はコードコメントのみ
- tests / pbt / fuzz にスレッド間移動 (thread::spawn + mpsc 等) を検証するテストが存在しない
- 既存の `test_roundtrip_decoder_threads` は `DecoderConfig.threads = Some(2)` の設定テストであり、スレッド間移動とは無関係

## 設計方針

- Encoder は `g_threads = Some(4)`、Decoder は `threads = Some(4)` を指定して、libaom の内部ワーカースレッドを生成した状態でスレッド間移動させる。指定しないと内部ワーカースレッドが生成されず、コメントの主張 (「内部スレッドはプロセス全体で有効な同期プリミティブで通信しており、移動後も正しく動作する」) を検証できない
- ワーカースレッドを生成・動作させた状態で移動するため、移動元のスレッドでエンコード・デコードを開始してから mpsc で送信する
- `&mut self` の排他取得中は移動できないため、動作中の移動は検証対象外
- 移動時点では next_frame ドレインを完了させた状態 (未ドレインのフレームがない状態) で送信する
- エンコーダーは realtime モード (g_lag_in_frames = 0) を使う。GoodQuality デフォルト (lag = 19) ではバッファリングによりエンコードしたフレームが即座にパケットとして出力されず、移動前の検証が空振りする
- 前提: libaom の内部ワーカースレッドは `CONFIG_MULTITHREAD=1` (libaom の CMake デフォルト ON) で生成される
- assert / expect メッセージは日本語で書く (テストのログメッセージを日本語にする規約)

## 完了条件

- `g_threads` / `threads` を指定して libaom の内部ワーカースレッドを生成した状態で、スレッド間移動後にエンコード・デコードが正常に動作するテストが追加され、通ること

## 解決方法

- `tests/send_across_threads.rs` を新規作成し、スレッド間移動時にエンコード・デコードが正常動作することを検証するテストを追加した
  - `test_encoder_send_across_threads`: スレッド A で `Encoder::new` (g_threads = Some(4)、row_mt + タイル有効) → 数フレーム encode → mpsc で送信 → メインスレッドで残りの encode → finish が正常動作することを検証。全パケットをデコードして入力フレーム数と一致することと、各フレームの幅・高さ・Y プレーン非空を確認する
  - `test_decoder_send_across_threads`: 事前エンコード (タイル付き) したパケットを用意し、スレッド A で `Decoder::new` (threads = Some(4)) → 一部パケット decode → mpsc で送信 → メインスレッドで残りの decode → finish が正常動作することを検証。デコードフレーム数が入力フレーム数と一致することと、各フレームの幅・高さ・Y プレーン非空を確認する
- libaom の内部ワーカースレッドが実際に並列処理するよう、エンコーダーに row_mt とタイル設定を有効にした
- 子スレッドの panic を検出できるよう、`JoinHandle` の `join` で結果を検査する構成にした
