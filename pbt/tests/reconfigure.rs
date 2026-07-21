//! reconfigure の性質を検証する PBT
//!
//! 対応する `src/` モジュールは無いため、ファイル名に `prop_` プレフィックスは付けない。

use proptest::prelude::*;
use shiguredo_aom::{
    EncodeOptions, Encoder, EncoderConfig, ImageData, ImageFormat, RateControlMode,
    ReconfigureParams, Usage,
};

fn generate_dummy_i420(
    width: usize,
    height: usize,
    frame_index: usize,
) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let y_size = width * height;
    let uv_size = width.div_ceil(2) * height.div_ceil(2);
    let mut y = vec![0u8; y_size];
    for row in 0..height {
        for col in 0..width {
            y[row * width + col] = ((col + row + frame_index * 7) % 256) as u8;
        }
    }
    (y, vec![128u8; uv_size], vec![128u8; uv_size])
}

fn realtime_cbr(width: u32, height: u32, bitrate: u32) -> EncoderConfig {
    let mut config = EncoderConfig::new(width, height, ImageFormat::I420);
    config.g_usage = Usage::Realtime;
    config.rc_end_usage = RateControlMode::Cbr;
    config.rc_target_bitrate = bitrate;
    config.cpu_used = Some(8);
    config
}

fn encode_count(encoder: &mut Encoder, width: u32, height: u32, frames: usize) -> usize {
    let options = EncodeOptions {
        force_keyframe: false,
    };
    let mut count = 0;
    for i in 0..frames {
        let (y, u, v) = generate_dummy_i420(width as usize, height as usize, i);
        let image = ImageData::I420 {
            y: &y,
            u: &u,
            v: &v,
        };
        encoder
            .encode(&image, &options)
            .expect("encode must succeed");
        while let Some(encoded) = encoder.next_frame() {
            let _ = encoded.data().expect("encoded frame data must be readable");
            count += 1;
        }
    }
    encoder.finish().expect("finish must succeed");
    while let Some(encoded) = encoder.next_frame() {
        let _ = encoded.data().expect("encoded frame data must be readable");
        count += 1;
    }
    count
}

proptest! {
    /// 任意の妥当なビットレートで midstream reconfigure してもエンコードが完走し、
    /// 入力フレーム数ぶんの出力が得られること
    #[test]
    fn reconfigure_preserves_frame_count(
        initial_bitrate in 200u32..5000,
        new_bitrate in 200u32..5000,
        switch_at in 1usize..6,
        trailing in 1usize..6,
    ) {
        let width = 160u32;
        let height = 120u32;
        let mut encoder = Encoder::new(realtime_cbr(width, height, initial_bitrate))
            .expect("encoder creation must succeed");

        let options = EncodeOptions {
            force_keyframe: false,
        };
        let mut produced = 0usize;

        for i in 0..switch_at {
            let (y, u, v) = generate_dummy_i420(width as usize, height as usize, i);
            let image = ImageData::I420 {
                y: &y,
                u: &u,
                v: &v,
            };
            encoder
                .encode(&image, &options)
                .expect("encode before reconfigure must succeed");
            while let Some(encoded) = encoder.next_frame() {
                let _ = encoded.data().expect("encoded frame data must be readable");
                produced += 1;
            }
        }

        encoder
            .reconfigure(ReconfigureParams {
                rc_target_bitrate: Some(new_bitrate),
            })
            .expect("reconfigure with in-range bitrate must succeed");

        for i in 0..trailing {
            let (y, u, v) =
                generate_dummy_i420(width as usize, height as usize, switch_at + i);
            let image = ImageData::I420 {
                y: &y,
                u: &u,
                v: &v,
            };
            encoder
                .encode(&image, &options)
                .expect("encode after reconfigure must succeed");
            while let Some(encoded) = encoder.next_frame() {
                let _ = encoded.data().expect("encoded frame data must be readable");
                produced += 1;
            }
        }

        encoder.finish().expect("finish must succeed");
        while let Some(encoded) = encoder.next_frame() {
            let _ = encoded.data().expect("encoded frame data must be readable");
            produced += 1;
        }

        prop_assert_eq!(produced, switch_at + trailing);
    }

    /// 同じビットレートで連続 reconfigure してもエンコードが完走すること (冪等性の緩い形)
    #[test]
    fn reconfigure_same_bitrate_is_safe(bitrate in 200u32..5000, repeats in 1usize..4) {
        let width = 160u32;
        let height = 120u32;
        let mut encoder = Encoder::new(realtime_cbr(width, height, bitrate))
            .expect("encoder creation must succeed");

        for _ in 0..repeats {
            encoder
                .reconfigure(ReconfigureParams {
                    rc_target_bitrate: Some(bitrate),
                })
                .expect("idempotent reconfigure must succeed");
        }

        let produced = encode_count(&mut encoder, width, height, 3);
        prop_assert_eq!(produced, 3);
    }
}
