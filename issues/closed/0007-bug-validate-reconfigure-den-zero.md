# reconfigure に den==0 バリデーションを追加する

Created: 2026-05-10
Completed: 2026-05-10
Model: deepseek v4-pro

## 概要

`ReconfigureParams::g_timebase` の `den` フィールドが 0 の場合のバリデーションがなく、libaom 内部で除算ゼロが発生する危険がある。`reconfigure()` で事前にチェックし、エラーを返すようにする。

## 根拠

`src/lib.rs:2008-2011`:

```rust
if let Some(v) = params.g_timebase {
    self.cfg.g_timebase.num = v.num as c_int;
    self.cfg.g_timebase.den = v.den as c_int;
}
```

`den = 0` のタイムベースを `aom_codec_enc_config_set()` に渡すと、libaom 内部で除算ゼロが発生する。

なお `num` については、`c_int` へのキャスト (`i32`) であり、負数を含めて libaom が受け入れるため、本 issue ではチェック対象外とする。

## 再現手順

1. `Encoder` を作成する
2. `reconfigure(ReconfigureParams { g_timebase: Some(AomRational { num: 1, den: 0 }), ..Default::default() })` を呼ぶ
3. libaom 内部で除算ゼロが発生する

## 修正方針

`reconfigure()` の `iter` チェック直後 (`src/lib.rs:2000` の次行) にバリデーションを追加する。`iter` チェックの後に置くことで、エラーの優先順位を `encode` / `finish` / `reconfigure` 本体のガードと揃える。

```rust
if let Some(v) = params.g_timebase {
    if v.den == 0 {
        return Err(Error::with_reason(
            sys::aom_codec_err_t_AOM_CODEC_INVALID_PARAM,
            "shiguredo_aom::Encoder::reconfigure",
            "g_timebase denominator must not be zero",
        ));
    }
}
```

`AomRational` は `Copy` を derive している (`src/lib.rs:623`) ため、`ref` パターンは不要である。

### 対象ファイル

- `src/lib.rs` — `reconfigure()` メソッドにバリデーションを追加

## テスト戦略

`tests/test_roundtrip.rs` に以下を追加する:

```rust
#[test]
fn test_reconfigure_rejects_zero_denominator() {
    let config = realtime_config(320, 240, RateControlMode::Cbr);
    let mut encoder = Encoder::new(config).expect("failed to create encoder");

    let result = encoder.reconfigure(ReconfigureParams {
        g_timebase: Some(AomRational { num: 1, den: 0 }),
        ..Default::default()
    });
    assert!(result.is_err());
}
```

## 他 issue との関係

- **0005** (`bug-fix-reconfigure-state-inconsistency`) — 同一関数を修正するが、本 issue のバリデーションは copy-first 修正の前（`params` 参照段階）で動作するため、適用順序による影響はない。どちらを先に適用してもよい
- **0006** (`bug-remove-gw-gh-from-reconfigure`) — `reconfigure()` の行番号が変わるため、本 issue の修正位置 (`2000` の次行) は 0006 の後にずれる可能性がある。0006 より後に適用することを推奨する
- 修正後は `CHANGES.md` の `## develop` に `[FIX]` エントリを追加する

## 解決方法

`0012-enh-add-libwebrtc-style-reconfigure` の方針採用により、本 issue は不要となった。

libwebrtc の AV1 エンコーダー実装は `g_timebase` をエンコーダー初期化時に `{1, 90000}` で固定し、ランタイムでは動かさない。フレームレート変動はエンコード時の duration（PTS 差分）で表現する。aom-rs もこの運用に倣う方針を採るため、`ReconfigureParams::g_timebase` フィールド自体を削除する（0006 のスコープに統合）。フィールドが消えれば `den == 0` を弾くバリデーションは存在自体が不要になる。

実コードでの対応は 0006 側で行う。
