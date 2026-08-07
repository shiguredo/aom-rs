# Encoder / Decoder のスレッド間移動検証テストを追加する

- Created: 2026-08-02
- Completed: {YYYY-MM-DD}
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

- テストは `tests/send_across_threads.rs` に新規作成する
- テスト 1 (エンコーダー): スレッド A で `Encoder::new` (g_threads = Some(4)、realtime モード) → 数フレーム encode (ワーカースレッドを生成・動作させる) → next_frame ドレイン → mpsc で送信 → メインスレッドで残りの encode → next_frame ドレイン → finish → finish 後の next_frame ドレイン が正常動作することを検証。移動前のパケット出力はスレッド A 内で確認し、移動後のエンコードでも 1 つ以上のパケットが出力されることと、finish が成功することを確認する
- テスト 2 (デコーダー): メインスレッドで事前にエンコード (realtime モード) したパケットを用意し、スレッド A で `Decoder::new` (threads = Some(4)) → decode と next_frame ドレインを交互に数パケット分実行 → mpsc で送信 → メインスレッドで残りの decode と next_frame ドレインを交互に実行 → finish → finish 後の next_frame ドレイン が正常動作することを検証。デコード結果のフレーム数が入力フレーム数と一致すること (finish 後のドレイン分を含む) と、各フレームの幅・高さが入力フレームの解像度と一致し、Y プレーンが非空であることを確認する
