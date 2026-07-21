# CHANGES.md の種別順序と規約違反を修正する

Created: 2026-07-21
Priority: Medium
Polished: 2026-07-21
Model: Qwen Code

## 概要

CHANGES.md の develop / misc セクションの種別順序が AGENTS.md の規約に違反している。また、build.rs に日本語ログメッセージ、テストに日本語 panic/assert メッセージがある。

## 根拠

### CHANGES.md develop セクションの順序違反

現在の順序: `[CHANGE]` x3 → `[ADD]` x7 → `[UPDATE]` x1 → `[FIX]` x2
規約の要求: UPDATE → ADD → CHANGE → FIX の順（AGENTS.md「エントリは種別の順番を守って記載すること」）

### CHANGES.md misc セクションの順序違反

現在の順序: `[ADD]` x4 → `[UPDATE]` x3
規約の要求: UPDATE → ADD の順

### CHANGES.md misc エントリの「〜する」形式違反

- `- [ADD] examples/midstream_reconfigure.rs を追加し、30fps エンコード途中でビットレートを切り替える典型パターンを示す`（「示す」で終わっている）
- `- [UPDATE] KeyframeMode から libaom の aom_kf_mode 定数へのマッピングを map_kf_mode private fn に切り出す`（「切り出す」で終わっている）

### build.rs の日本語ログメッセージ

`build.rs:119`: `eprintln!("prebuilt ライブラリをダウンロード中: {}", archive_url);`
AGENTS.md「ログメッセージは全て英語」に違反。同ファイル 200 行目は英語で不統一。

### テストの日本語 panic/assert メッセージ

- `tests/test_roundtrip.rs:846`: `panic!("エンコードプロファイルが Unsupported になっている");`
- `tests/test_roundtrip.rs:1229`: `"force_keyframe = true で reconfigure 直後のフレームがキーフレームとして出力されなかった"`

AGENTS.md「エラーメッセージは全て英語」に違反。

### build.rs / lib.rs の英語コメント

- `build.rs:52`: `// See also: https://docs.rs/about/builds`
- `src/lib.rs:1660`: `// NOTE: これ以降の操作に失敗しても ctx は Drop によって確実に解放される`

AGENTS.md「コメントは全て日本語」に違反。

## 修正方針

- CHANGES.md のエントリを UPDATE → ADD → CHANGE → FIX の順に並び替える
- 「〜する」形式に統一する
- build.rs:119 のログメッセージを英語に変更する
- テストの panic/assert メッセージを英語に変更する
- 英語コメントを日本語に変更する

## 後方互換

ドキュメント・コメント・ログメッセージの修正のみ。コードの動作に変更なし。
