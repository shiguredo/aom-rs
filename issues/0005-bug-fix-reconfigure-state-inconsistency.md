# reconfigure() 失敗時に self.cfg と libaom 実状態が乖離する

Created: 2026-05-10
Model: deepseek v4-pro

## 概要

`Encoder::reconfigure()` が `self.cfg` を書き換えた**後**で `aom_codec_enc_config_set()` を呼び出している。この呼び出しが失敗した場合、Rust 側の `self.cfg` は書き換わっているが libaom 内部 (`self.ctx`) は変更前のままになり、状態が乖離する。修正後も誤った設定で動作するため、リトライや状態確認が不可能になる。

## 根拠

`src/lib.rs:2002-2017`:

```rust
if let Some(v) = params.g_w {
    self.cfg.g_w = v as _;   // self.cfg を先に書き換える
}
// ... 他フィールド (g_h, g_timebase, rc_target_bitrate) も同様 ...
let code = unsafe { sys::aom_codec_enc_config_set(&mut self.ctx, &self.cfg) };
Error::check(code, "aom_codec_enc_config_set", Some(&self.ctx))?;
```

`aom_codec_enc_config_set()` のシグネチャは `const` ポインタ (`*const aom_codec_enc_cfg_t`) を受け取り、設定の読み取りのみを行う。失敗時は libaom 内部状態が変更されないため、Rust 側だけが書き換わって不整合になる。

## 再現手順

1. `Encoder` を作成し、1 フレーム以上正常にエンコードする
2. `reconfigure()` に不正な値を渡す。libaom が実際にエラーを返す値は要検証だが、例として `rc_target_bitrate: Some(u32::MAX)` や、`rc_target_bitrate` と `rc_buf_sz` / `rc_buf_initial_sz` / `rc_buf_optimal_sz` の比率制約に違反する組み合わせが候補
3. `reconfigure()` が `Err` を返した直後、`self.cfg.rc_target_bitrate` は `u32::MAX` に書き換わっている
4. 以降の `encode()` が、変更前の設定でエンコード動作を続けるが、Rust 側の `self.cfg` は破壊されており、再度 `reconfigure()` を呼ぶと壊れた値がベースになる

## 修正方針

`self.cfg` を直接書き換えず、`self.cfg` の内容を別の `aom_codec_enc_cfg` に複製し、複製を書き換えて `aom_codec_enc_config_set()` に渡す。成功した場合のみ `self.cfg` を差し替える。

### 実装上の注意

`sys::aom_codec_enc_cfg` は bindgen 生成型であり、`Copy` トレイトが実装されているとは限らない（`build.rs` の bindgen 設定に `derive_copy(true)` は指定されていない）。`let mut cfg = self.cfg;` はコンパイルエラーになる可能性が高い。

代わりに `std::ptr::read` による unsafe なビットコピーを用いる:

```rust
// 変更前の cfg を複製する
// 安全性: aom_codec_enc_cfg_t は内部にポインタやリソースを持たない POD 型であり、
// ビットコピーが安全である
let mut cfg: sys::aom_codec_enc_cfg = unsafe { std::ptr::read(&self.cfg) };

// cfg を書き換え (各フィールドの代入は既存コードと同じ)
if let Some(v) = params.g_w {
    cfg.g_w = v as _;
}
// ... g_h, g_timebase, rc_target_bitrate も同様 ...

let code = unsafe { sys::aom_codec_enc_config_set(&mut self.ctx, &cfg) };
Error::check(code, "aom_codec_enc_config_set", Some(&self.ctx))?;

// 成功時のみ差し替え
self.cfg = cfg;
```

### 注意

- `aom_codec_enc_config_set()` が内部的に部分的な状態変更をしてから失敗する可能性については、libaom の実装で `cfg` を `const` ポインタで受けていることから、libaom 側の不整合は発生しないと想定する。もし libaom にそのような実装があれば、それは libaom 側のバグとして扱う

## テスト戦略

### 単体テスト（`tests/test_roundtrip.rs` に追加）

```rust
#[test]
fn test_reconfigure_state_unchanged_on_failure() {
    // reconfigure 失敗後に self.cfg が変更前の値のままであること
    // かつ、その後の encode が正常に動作することを確認する
}
```

### PBT（`pbt/` に追加、0008 に依存）

- 「任意の `ReconfigureParams` 入力に対し、成功時は cfg が正しく更新され、失敗時は cfg が不変である」の性質を検証する

### 回帰確認

- 既存の `test_reconfigure_target_bitrate_midstream`、`test_reconfigure_immediately_after_new`、`test_reconfigure_empty_params_is_noop` が通過すること

## 依存 issue

- **0006** (`bug-remove-gw-gh-from-reconfigure`) が先に適用される場合、本 issue の修正方針コードから `g_w` / `g_h` の代入を除去する必要がある。本 issue は 0006 の**後**に適用することを推奨する（不要なコード修正を避けるため）
- **0008** (`enh-add-reconfigure-pbt`) で PBT が導入されるまでは単体テストのみで対応する
- 修正後は `CHANGES.md` の `## develop` に `[FIX]` エントリを追加する（既存 `[ADD]` より後に記載する）

## 後方互換

内部実装の修正のみ。`reconfigure()` のシグネチャ・公開 API は変更なし。
