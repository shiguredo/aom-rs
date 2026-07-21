use shiguredo_aom::{
    Decoder, DecoderConfig, EncodeOptions, Encoder, EncoderConfig, ImageData, ImageFormat,
    RateControlMode, ReconfigureParams, Usage,
};

// ============================================================================
// フレーム生成ヘルパー
// ============================================================================

/// ダミー I420 フレームを生成する
///
/// Y プレーンはフレーム番号に応じたグラデーション、UV プレーンは 128 固定。
pub(crate) fn generate_dummy_i420(
    width: usize,
    height: usize,
    frame_index: usize,
) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let y_size = width * height;
    let uv_width = width.div_ceil(2);
    let uv_height = height.div_ceil(2);
    let uv_size = uv_width * uv_height;

    let mut y = vec![0u8; y_size];
    for row in 0..height {
        for col in 0..width {
            y[row * width + col] = ((col + row + frame_index * 7) % 256) as u8;
        }
    }

    let u = vec![128u8; uv_size];
    let v = vec![128u8; uv_size];

    (y, u, v)
}

/// SMPTE カラーバー風の I420 フレームを生成する
///
/// 7 色の縦ストライプ（白/黄/シアン/緑/マゼンタ/赤/青）を
/// BT.601 で YUV に変換し I420 形式で返す。
pub(crate) fn generate_colorbar_i420(width: usize, height: usize) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    // SMPTE カラーバーの RGB 値（白/黄/シアン/緑/マゼンタ/赤/青）
    let bars: [(u8, u8, u8); 7] = [
        (235, 235, 235), // 白
        (235, 235, 16),  // 黄
        (16, 235, 235),  // シアン
        (16, 235, 16),   // 緑
        (235, 16, 235),  // マゼンタ
        (235, 16, 16),   // 赤
        (16, 16, 235),   // 青
    ];

    let y_size = width * height;
    let uv_width = width.div_ceil(2);
    let uv_height = height.div_ceil(2);
    let uv_size = uv_width * uv_height;

    let mut y_plane = vec![0u8; y_size];
    let mut u_plane = vec![128u8; uv_size];
    let mut v_plane = vec![128u8; uv_size];

    for row in 0..height {
        for col in 0..width {
            let bar_index = col * 7 / width;
            let (r, g, b) = bars[bar_index];

            // BT.601 RGB -> YCbCr
            let rf = r as f64;
            let gf = g as f64;
            let bf = b as f64;
            let yv = (0.257 * rf + 0.504 * gf + 0.098 * bf + 16.0).clamp(16.0, 235.0) as u8;
            y_plane[row * width + col] = yv;

            // UV は 2x2 ブロック単位（左上ピクセルで代表する）
            if row % 2 == 0 && col % 2 == 0 {
                let u = (-0.148 * rf - 0.291 * gf + 0.439 * bf + 128.0).clamp(16.0, 240.0) as u8;
                let v = (0.439 * rf - 0.368 * gf - 0.071 * bf + 128.0).clamp(16.0, 240.0) as u8;
                let uv_row = row / 2;
                let uv_col = col / 2;
                u_plane[uv_row * uv_width + uv_col] = u;
                v_plane[uv_row * uv_width + uv_col] = v;
            }
        }
    }

    (y_plane, u_plane, v_plane)
}

// ============================================================================
// 品質計測ヘルパー
// ============================================================================

/// Y プレーン同士の PSNR を計算する（dB）
///
/// 値が大きいほど入力と出力が近い。一般に 30dB 以上あれば視覚的に良好。
pub(crate) fn psnr_y(original: &[u8], decoded: &[u8], width: usize, height: usize) -> f64 {
    let y_size = width * height;
    assert!(original.len() >= y_size);
    assert!(decoded.len() >= y_size);

    let mut mse_sum: f64 = 0.0;
    for i in 0..y_size {
        let diff = original[i] as f64 - decoded[i] as f64;
        mse_sum += diff * diff;
    }
    let mse = mse_sum / y_size as f64;
    if mse == 0.0 {
        return f64::INFINITY;
    }
    10.0 * (255.0_f64 * 255.0 / mse).log10()
}

// ============================================================================
// エンコード / デコードヘルパー
// ============================================================================

/// デコード結果の Y プレーンをストライド無しで抽出する
///
/// libaom のデコード結果はストライドが幅と一致するとは限らないため、
/// 行ごとに width 分だけコピーして詰める。
pub(crate) fn extract_y_plane(frame: &shiguredo_aom::DecodedFrame<'_>) -> Vec<u8> {
    let width = frame.width();
    let height = frame.height();
    let stride = frame.y_stride().expect("failed to get Y stride");
    let y_data = frame.y_plane().expect("failed to get Y plane");
    let mut y = Vec::new();
    for row in 0..height {
        y.extend_from_slice(&y_data[row * stride..row * stride + width]);
    }
    y
}

/// エンコードしてフレーム単位のビットストリームを返すヘルパー
///
/// AV1 デコーダーはフレーム単位でデータを受け取る必要があるため、
/// エンコード済みフレームごとに分割して返す。
pub(crate) fn encode_frames(
    config: EncoderConfig,
    frames: &[(Vec<u8>, Vec<u8>, Vec<u8>)],
) -> Vec<Vec<u8>> {
    let mut encoder = Encoder::new(config).expect("failed to create encoder");
    let options = EncodeOptions {
        force_keyframe: false,
    };
    let mut encoded_packets = Vec::new();

    for (y, u, v) in frames {
        let image = ImageData::I420 { y, u, v };
        encoder.encode(&image, &options).expect("failed to encode");
        while let Some(encoded) = encoder.next_frame() {
            encoded_packets.push(encoded.data().expect("failed to get encoded data").to_vec());
        }
    }

    encoder.finish().expect("failed to finish");
    while let Some(encoded) = encoder.next_frame() {
        encoded_packets.push(encoded.data().expect("failed to get encoded data").to_vec());
    }

    encoded_packets
}

/// デコードしてフレーム情報一覧を返すヘルパー
///
/// デコード結果の Y プレーン、幅、高さを返す。
pub(crate) fn decode_frames(packets: &[Vec<u8>]) -> Vec<(Vec<u8>, usize, usize)> {
    decode_frames_with_config(DecoderConfig::default(), packets)
}

/// 指定したデコーダー設定でデコードしてフレーム情報一覧を返すヘルパー
pub(crate) fn decode_frames_with_config(
    config: DecoderConfig,
    packets: &[Vec<u8>],
) -> Vec<(Vec<u8>, usize, usize)> {
    let mut decoder = Decoder::new(config).expect("failed to create decoder");
    let mut decoded = Vec::new();

    for packet in packets {
        decoder.decode(packet).expect("failed to decode");
        while let Some(frame) = decoder.next_frame() {
            decoded.push((extract_y_plane(&frame), frame.width(), frame.height()));
        }
    }

    decoder.finish().expect("failed to finish");
    while let Some(frame) = decoder.next_frame() {
        decoded.push((extract_y_plane(&frame), frame.width(), frame.height()));
    }

    decoded
}

// ============================================================================
// ラウンドトリップヘルパー
// ============================================================================

/// エンコード→デコードのラウンドトリップを検証するヘルパー
pub(crate) fn roundtrip(
    config: EncoderConfig,
    input_frames: &[(Vec<u8>, Vec<u8>, Vec<u8>)],
) -> Vec<(Vec<u8>, usize, usize)> {
    let width = config.g_w as usize;
    let height = config.g_h as usize;
    let num_frames = input_frames.len();

    let packets = encode_frames(config, input_frames);
    assert!(!packets.is_empty(), "no encoded packets");

    let decoded_frames = decode_frames(&packets);

    assert_eq!(
        decoded_frames.len(),
        num_frames,
        "decoded {} frames, expected {num_frames}",
        decoded_frames.len()
    );
    for (i, (y, w, h)) in decoded_frames.iter().enumerate() {
        assert_eq!(*w, width, "decoded frame {i} width mismatch");
        assert_eq!(*h, height, "decoded frame {i} height mismatch");
        assert!(!y.is_empty(), "decoded frame {i} has empty Y plane");
    }

    decoded_frames
}

/// カラーバーを使ったラウンドトリップで PSNR を検証するヘルパー
///
/// 同一のカラーバーフレームを num_frames 回エンコードし、デコード後に
/// 元の Y プレーンとの PSNR が min_psnr_db 以上であることを確認する。
pub(crate) fn roundtrip_colorbar(config: EncoderConfig, num_frames: usize, min_psnr_db: f64) {
    let width = config.g_w as usize;
    let height = config.g_h as usize;

    let (y, u, v) = generate_colorbar_i420(width, height);
    let input_frames: Vec<(Vec<u8>, Vec<u8>, Vec<u8>)> = (0..num_frames)
        .map(|_| (y.clone(), u.clone(), v.clone()))
        .collect();

    let decoded_frames = roundtrip(config, &input_frames);

    for (i, (decoded_y, _, _)) in decoded_frames.iter().enumerate() {
        let psnr = psnr_y(&y, decoded_y, width, height);
        assert!(
            psnr >= min_psnr_db,
            "frame {i}: PSNR {psnr:.1} dB < {min_psnr_db} dB"
        );
    }
}

// ============================================================================
// 設定生成ヘルパー
// ============================================================================

/// Realtime モードのエンコーダー設定を生成するヘルパー
///
/// Realtime モードは lag_in_frames=0 で動作するため、
/// エンコードしたフレームが即座に出力される。
pub(crate) fn realtime_config(
    width: u32,
    height: u32,
    rate_control: RateControlMode,
) -> EncoderConfig {
    let mut config = EncoderConfig::new(width, height, ImageFormat::I420);
    config.g_usage = Usage::Realtime;
    config.rc_end_usage = rate_control;
    config.rc_target_bitrate = 1000;
    config.cpu_used = Some(8);
    config
}

/// GoodQuality モードのエンコーダー設定を生成するヘルパー
pub(crate) fn good_quality_config(
    width: u32,
    height: u32,
    rate_control: RateControlMode,
) -> EncoderConfig {
    let mut config = EncoderConfig::new(width, height, ImageFormat::I420);
    config.g_usage = Usage::GoodQuality;
    config.rc_end_usage = rate_control;
    config.rc_target_bitrate = 1000;
    config.cpu_used = Some(8);
    config.g_lag_in_frames = Some(0);
    config
}

/// AllIntra モードのエンコーダー設定を生成するヘルパー
pub(crate) fn all_intra_config(
    width: u32,
    height: u32,
    rate_control: RateControlMode,
) -> EncoderConfig {
    let mut config = EncoderConfig::new(width, height, ImageFormat::I420);
    config.g_usage = Usage::AllIntra;
    config.rc_end_usage = rate_control;
    config.rc_target_bitrate = 1000;
    config.cpu_used = Some(8);
    config
}

// ============================================================================
// reconfigure テスト用ヘルパー
// ============================================================================

/// libaom の `rc_target_bitrate` 受け入れ上限 (kbps)
///
/// 出典: 生成済み bindings の doc コメント (`Max allowed value is 2000000`)
pub(crate) const LIBAOM_RC_TARGET_BITRATE_MAX_KBPS: u32 = 2_000_000;

/// `rc_target_bitrate` だけを変更する [`ReconfigureParams`] を生成するヘルパー
pub(crate) fn rc_bitrate(kbps: u32) -> ReconfigureParams {
    ReconfigureParams {
        rc_target_bitrate: Some(kbps),
    }
}

/// ダミー I420 フレームをエンコードしてパケットを `packets` に積む
pub(crate) fn drive_dummy(
    encoder: &mut Encoder,
    options: &EncodeOptions,
    width: u32,
    height: u32,
    range: std::ops::Range<usize>,
    packets: &mut Vec<Vec<u8>>,
) {
    for i in range {
        let (y, u, v) = generate_dummy_i420(width as usize, height as usize, i);
        let image = ImageData::I420 {
            y: &y,
            u: &u,
            v: &v,
        };
        encoder.encode(&image, options).expect("failed to encode");
        while let Some(encoded) = encoder.next_frame() {
            packets.push(encoded.data().expect("failed to get encoded data").to_vec());
        }
    }
}

/// `finish()` を呼び、残りのパケットを `packets` に積む
pub(crate) fn drain_after_finish(encoder: &mut Encoder, packets: &mut Vec<Vec<u8>>) {
    encoder.finish().expect("failed to finish");
    while let Some(encoded) = encoder.next_frame() {
        packets.push(encoded.data().expect("failed to get encoded data").to_vec());
    }
}

pub(crate) fn total_bytes(packets: &[Vec<u8>]) -> u64 {
    packets.iter().map(|p| p.len() as u64).sum()
}
