//! 30fps エンコード途中でビットレートを切り替えるサンプル
//!
//! libwebrtc の AV1 エンコーダーラッパー (`LibaomAv1Encoder::SetRates`) と同じく、
//! エンコーダーを破棄せず [`Encoder::reconfigure`] で動的にビットレートを変更する
//! 典型パターンを示す。タイムベースは初期化時に 30fps (`{1, 30}`) で固定し、
//! ランタイムでは触らない。
//!
//! 実行例:
//!
//! ```sh
//! cargo run --example midstream_reconfigure
//! ```

use shiguredo_aom::{
    AomRational, EncodeOptions, Encoder, EncoderConfig, ImageData, ImageFormat, RateControlMode,
    ReconfigureParams, Usage,
};

const WIDTH: u32 = 320;
const HEIGHT: u32 = 240;
const FPS_DEN: i32 = 30;
const NUM_FRAMES: usize = 60;
const SWITCH_AT: usize = 30;
const INITIAL_BITRATE_KBPS: u32 = 500;
const SWITCHED_BITRATE_KBPS: u32 = 2000;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = EncoderConfig::new(WIDTH, HEIGHT, ImageFormat::I420);
    config.g_usage = Usage::Realtime;
    config.rc_end_usage = RateControlMode::Cbr;
    config.rc_target_bitrate = INITIAL_BITRATE_KBPS;
    config.cpu_used = Some(8);
    // タイムベースは初期化時にフレームレートに合わせて固定する
    config.g_timebase = AomRational {
        num: 1,
        den: FPS_DEN,
    };

    let mut encoder = Encoder::new(config)?;
    let options = EncodeOptions {
        force_keyframe: false,
    };

    let y_size = (WIDTH * HEIGHT) as usize;
    let uv_size = (WIDTH.div_ceil(2) * HEIGHT.div_ceil(2)) as usize;

    let mut total_bytes = [0usize; 2];
    let mut total_frames = [0usize; 2];

    for i in 0..NUM_FRAMES {
        if i == SWITCH_AT {
            // libwebrtc 方式: エンコーダーを再生成せず reconfigure だけで切り替える
            encoder.reconfigure(ReconfigureParams {
                rc_target_bitrate: Some(SWITCHED_BITRATE_KBPS),
            })?;
            println!("frame {i}: reconfigure rc_target_bitrate -> {SWITCHED_BITRATE_KBPS} kbps");
        }

        // ビットレート変化が見える程度のグラデーションを生成する
        let y: Vec<u8> = (0..y_size).map(|p| ((p + i * 7) % 256) as u8).collect();
        let u = vec![128u8; uv_size];
        let v = vec![128u8; uv_size];
        let image = ImageData::I420 {
            y: &y,
            u: &u,
            v: &v,
        };
        encoder.encode(&image, &options)?;

        let segment = if i < SWITCH_AT { 0 } else { 1 };
        while let Some(encoded) = encoder.next_frame() {
            let bytes = encoded.data()?.len();
            total_bytes[segment] += bytes;
            total_frames[segment] += 1;
        }
    }

    encoder.finish()?;
    while let Some(encoded) = encoder.next_frame() {
        let bytes = encoded.data()?.len();
        total_bytes[1] += bytes;
        total_frames[1] += 1;
    }

    for (i, &kbps) in [INITIAL_BITRATE_KBPS, SWITCHED_BITRATE_KBPS]
        .iter()
        .enumerate()
    {
        let frames = total_frames[i];
        let bytes = total_bytes[i];
        let avg = bytes.checked_div(frames).unwrap_or(0);
        println!(
            "segment {i}: target={kbps} kbps, frames={frames}, bytes={bytes}, avg/frame={avg}"
        );
    }

    Ok(())
}
