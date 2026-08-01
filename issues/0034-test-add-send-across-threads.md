# Encoder / Decoder のスレッド間移動検証テストを追加する

- Created: 2026-08-02
- Completed: {YYYY-MM-DD}
- Branch: feature/add-send-across-threads-test
- Polished: {YYYY-MM-DD}
- Reporter: @voluntas

## 目的

`unsafe impl Send` の安全性根拠がコメントのみで、実際にスレッド間移動して動作することを実証するテストが存在しない。unsafe コードの検証としては不十分である。

## 現状

- `Encoder` (unsafe impl Send) と `Decoder` (unsafe impl Send) を宣言している
- 安全性の根拠はコードコメント (libaom の内部ポインタはヒープまたは静的データでありスレッドアフィニティを持たない、`&mut self` による排他アクセスが libaom の要求する排他性と一致する等) のみ
- tests / pbt / fuzz にスレッド間移動 (thread::spawn + mpsc 等) を検証するテストが存在しない
- 既存の `test_roundtrip_decoder_threads` は `DecoderConfig.threads = Some(2)` の設定テストであり、スレッド間移動とは無関係

## 設計方針

`g_threads = Some(4)` を指定して libaom の内部ワーカースレッドを生成した状態でスレッド間移動させる。g_threads を指定しないと内部ワーカースレッドが生成されず、コメントの主張 (「内部スレッドはプロセス全体で有効な同期プリミティブで通信しており、移動後も正しく動作する」) を検証できない。

## 完了条件

- スレッド間移動後にエンコード・デコードが正常に動作するテストが追加され、通ること

## 解決方法

- テスト 1: スレッド A で `Encoder::new` (g_threads = Some(4)) → mpsc で送信 → メインスレッドで encode → next_frame ドレイン → finish が正常動作することを検証
- テスト 2: スレッド A で `Decoder::new` + 全パケット decode → 送信 → メインスレッドで next_frame ドレイン + finish が正常動作することを検証
- デコード結果のフレーム数一致まで確認する
