# canary.py のプロンプト・コメント・git 状態検証を修正する

Created: 2026-07-21
Completed: 2026-07-21
Priority: Low
Polished: 2026-07-21
Model: Qwen Code

## 概要

canary.py に 3 つの問題がある。

## 根拠

### (Y/n) プロンプトと空入力の扱いが矛盾

`canary.py:68-74`:

```python
confirmation: str = (
    input("Do you want to update the version? (Y/n): ").strip().lower()
)
if confirmation != "y":
    print("Version update canceled.")
    return None
```

`(Y/n)` 表記は「Enter（空入力）= Yes」の慣例だが、コードは空文字列 `""` を `"y"` と一致せずキャンセルとして扱う。Enter を押すと意に反してキャンセルされる。

### 関数コメントのコピペミス

`canary.py:97`（`git_commit_version`）と `canary.py:111`（`git_operations_after_build`）の両方が `# git コミット、タグ、プッシュを実行` という同一コメントを持つが、`git_commit_version` は commit のみ、`git_operations_after_build` は tag と push のみを行う。

### git 状態の事前検証がない

`git_commit_version()` と `git_operations_after_build()` は以下を検証しない:
- ワーキングツリーに未コミットの変更があるか
- タグが既に存在するか
- リモートとの差分

## 修正方針

- `confirmation not in ("y", "")` に修正する
- 関数コメントを実態に合わせる
- git 状態の事前検証を追加する（任意）
- 非対話モード（`--yes`）の追加を検討する（任意）

## 後方互換

ユーティリティスクリプトの修正のみ。クレートのコード・API に変更なし。

## 解決方法

- `(Y/n)` プロンプトの空入力を Yes として扱うよう `confirmation not in ("y", "")` に修正した
- `git_commit_version` のコメントを「git コミットを実行」に修正した
- `git_operations_after_build` のコメントを「git タグ付け、プッシュを実行」に修正した
