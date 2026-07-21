# README / SKILL.md のコード例とバージョン情報を修正する

Created: 2026-07-21
Completed: 2026-07-21
Priority: Medium
Polished: 2026-07-21
Model: Qwen Code

## 概要

README.md のコード例にコンパイルエラーがあり、SKILL.md のバージョン情報が古い。

## 根拠

### README エンコードコード例のコンパイルエラー

README.md「エンコード」セクション:

```rust
while let Some(frame) = encoder.next_frame() {
    let data = frame.data();
    let is_key = frame.is_keyframe();
    println!("encoded: {} bytes, keyframe: {}", data.len(), is_key);
}
```

`EncodedFrame::data()` のシグネチャは `pub fn data(&self) -> Result<&[u8], Error>`（`src/lib.rs:2337`）。`Result<&[u8], Error>` に `len()` メソッドはないため、`data.len()` はコンパイルエラーになる。SKILL.md では `frame.data()?` と正しく記載。

### README デコードコード例の Result 未処理

README.md「デコード」セクション:

```rust
let y = frame.y_plane();
let u = frame.u_plane();
let v = frame.v_plane();
```

`y_plane()` 等は `Result<&[u8], Error>` を返す（`src/lib.rs:384,389,394`）。`?` なしで代入しており、ユーザーが `&[u8]` を直接返すと誤認する。SKILL.md では `frame.y_plane()?` と正しい。

### SKILL.md のバージョン情報不一致

SKILL.md「バージョン情報」セクション:
- `バージョン: 2026.1.0` → 実際は `2026.2.0-canary.2`（Cargo.toml）
- `libaom バージョン: v3.13.2` → 実際は `v3.14.1`（Cargo.toml）

### SKILL.md の Encoder Send 誤記載

SKILL.md「落とし穴 / 注意事項」セクション:
- 「`Encoder` はデフォルトで `Send` ではない」→ 実際は `src/lib.rs:2315` に `unsafe impl Send for Encoder {}` が存在する

### README 動作要件の不整合

- CI は `ubuntu-26.04` / `ubuntu-26.04-arm` でテストしているが README に記載なし
- README の "Windows 11 x86_64" は CI でテストした証拠がない（CI は `windows-2025` のみ）

### README 環境変数テーブルに DOCS_RS 未記載

build.rs は `DOCS_RS` 環境変数で分岐しているが、README の環境変数テーブルには `LIBAOM_TARGET` しか記載されていない。

## 修正方針

- README のコード例を `?` を使った形に修正する
- SKILL.md のバージョン情報・Send 記載・動作要件を更新する
- README の動作要件を CI / release.yml と整合させる
- README の環境変数テーブルに `DOCS_RS` を追加する

## 後方互換

ドキュメントの修正のみ。コードの変更なし。

## 解決方法

- README のエンコードコード例で `frame.data()` を `frame.data()?` に修正した
- README のデコードコード例で `y_plane()` 等に `?` を追加した
- SKILL.md のバージョン情報を 2026.2.0-canary.2 / v3.14.1 に更新した
- SKILL.md の Encoder Send 記載を正しい内容に修正した
