//! 任意入力に対するデコーダーのパニック耐性を検証する fuzz ターゲット
#![no_main]

use libfuzzer_sys::fuzz_target;
use shiguredo_aom::{Decoder, DecoderConfig};

fuzz_target!(|data: &[u8]| {
    let Ok(mut decoder) = Decoder::new(DecoderConfig::default()) else {
        return;
    };

    // 破損データでも Result で失敗するだけでパニックしないこと
    let _ = decoder.decode(data);
    while decoder.next_frame().is_some() {}

    let _ = decoder.finish();
    while decoder.next_frame().is_some() {}
});
