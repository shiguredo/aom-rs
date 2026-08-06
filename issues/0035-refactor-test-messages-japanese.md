# テストの assert / panic メッセージを日本語に統一する

- Created: 2026-08-06
- Completed: {YYYY-MM-DD}
- Branch: feature/refactor-test-messages-japanese
- Polished: {YYYY-MM-DD}

## 目的

AGENTS.md の「テストのログメッセージは全て日本語にすること」という規約を守るため、テストコードの assert / panic / expect メッセージを日本語に統一する。

## 現状

AGENTS.md には「テストのログメッセージは全て日本語にすること」と明記されているが、`tests/roundtrip.rs` / `tests/helpers/mod.rs` / `tests/reconfigure.rs` の `assert!` / `assert_eq!` / `expect!` / `panic!` メッセージは全て英語である。新規に追加されるテストも既存の英語メッセージを踏襲しており、規約違反が拡大し続けている。

対象はテストコード (`tests/` 配下) のみで、`src/` のエラーメッセージは「ログメッセージは全て英語にすること」の規約に従い英語のままとする。

## 設計方針

- `tests/` 配下の `assert!` / `assert_eq!` / `expect!` / `panic!` / `unreachable!` のメッセージを全て日本語に書き換える
- `src/` のエラーメッセージ・ログメッセージは英語のまま変更しない (別規約の対象)
- テスト関数名・テストコメントは現状のまま (関数名は英語、コメントは日本語で既に準拠)
- 挙動の変更は一切行わない (メッセージ文字列の言語のみ変更)

## 完了条件

- `tests/` 配下の全テストコードの `assert!` / `assert_eq!` / `expect!` / `panic!` / `unreachable!` メッセージが全て日本語であること
- `src/` のエラーメッセージ・ログメッセージが英語のままであること
- 全テストが通ること (回帰なし)

## 解決方法

- `tests/roundtrip.rs` / `tests/helpers/mod.rs` / `tests/reconfigure.rs` の assert / panic / expect メッセージを日本語に書き換える
- 検証: 書き換え後に全テストを実行し、メッセージの言語以外の差分がないことを `git diff` で確認する
