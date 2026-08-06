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

/// 16-bit カラーバー風フレームを生成する（u16 平面）
///
/// SMPTE カラーバーの 7 色の縦ストライプ（白/黄/シアン/緑/マゼンタ/赤/青）を
/// BT.601 で YUV 変換し、ビット深度に応じたスケールで 16-bit 値として返す。
/// サブサンプリングはフォーマットに従う（4:2:0 は 2x2 ブロック、4:2:2 は
/// 横 2 ピクセル、4:4:4 はピクセル単位）。UV はサブサンプリング単位の
/// 代表ピクセル（左上）で計算する。
pub(crate) fn generate_colorbar_16bit(
    width: usize,
    height: usize,
    format: ImageFormat,
    bit_depth: usize,
) -> (Vec<u16>, Vec<u16>, Vec<u16>) {
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

    let (uv_width, uv_height) = match format {
        ImageFormat::I42016 => (width.div_ceil(2), height.div_ceil(2)),
        ImageFormat::I42216 => (width.div_ceil(2), height),
        ImageFormat::I44416 => (width, height),
        _ => panic!("generate_colorbar_16bit: unsupported format {format:?}"),
    };
    let scale = ((1u32 << bit_depth) - 1) as f64 / 255.0;

    let mut y_plane = vec![0u16; width * height];
    let mut u_plane = vec![0u16; uv_width * uv_height];
    let mut v_plane = vec![0u16; uv_width * uv_height];

    for row in 0..height {
        for col in 0..width {
            let bar_index = col * 7 / width;
            let (r, g, b) = bars[bar_index];

            let rf = r as f64;
            let gf = g as f64;
            let bf = b as f64;
            let yv = (0.257 * rf + 0.504 * gf + 0.098 * bf + 16.0).clamp(16.0, 235.0) * scale;
            y_plane[row * width + col] = yv as u16;

            // UV はサブサンプリング単位（4:2:0 は 2x2、4:2:2 は横 2、4:4:4 は 1x1）の
            // 代表ピクセル（左上）で計算する
            let uv_cell = match format {
                ImageFormat::I44416 => Some((col, row)),
                ImageFormat::I42216 if col % 2 == 0 => Some((col / 2, row)),
                ImageFormat::I42016 if col % 2 == 0 && row % 2 == 0 => Some((col / 2, row / 2)),
                ImageFormat::I42216 | ImageFormat::I42016 => None,
                _ => unreachable!("generate_colorbar_16bit: unsupported format {format:?}"),
            };
            if let Some((uv_col, uv_row)) = uv_cell {
                let u = (-0.148 * rf - 0.291 * gf + 0.439 * bf + 128.0).clamp(16.0, 240.0) * scale;
                let v = (0.439 * rf - 0.368 * gf - 0.071 * bf + 128.0).clamp(16.0, 240.0) * scale;
                u_plane[uv_row * uv_width + uv_col] = u as u16;
                v_plane[uv_row * uv_width + uv_col] = v as u16;
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

/// 16-bit Y プレーン同士の PSNR を計算する（dB）
///
/// 最大値はビット深度に応じた `(1 << bit_depth) - 1` を使う。
pub(crate) fn psnr_y_16bit(
    original: &[u16],
    decoded: &[u16],
    width: usize,
    height: usize,
    bit_depth: usize,
) -> f64 {
    let y_size = width * height;
    assert!(original.len() >= y_size);
    assert!(decoded.len() >= y_size);

    let max_value = ((1u32 << bit_depth) - 1) as f64;
    let mut mse_sum: f64 = 0.0;
    for i in 0..y_size {
        let diff = original[i] as f64 - decoded[i] as f64;
        mse_sum += diff * diff;
    }
    let mse = mse_sum / y_size as f64;
    if mse == 0.0 {
        return f64::INFINITY;
    }
    10.0 * (max_value * max_value / mse).log10()
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

/// デコード結果の 16-bit Y プレーンをストライド無しで抽出する
///
/// libaom の 16-bit プレーンは各ピクセルがリトルエンディアン 2 バイトで
/// 格納される。ストライドがピクセル単位の幅と一致するとは限らないため、
/// 行ごとに width ピクセル分だけコピーして詰める。
pub(crate) fn extract_y_plane_16bit(frame: &shiguredo_aom::DecodedFrame<'_>) -> Vec<u16> {
    let width = frame.width();
    let height = frame.height();
    let stride = frame.y_stride().expect("failed to get Y stride");
    let y_data = frame.y_plane().expect("failed to get Y plane");
    let mut y = Vec::new();
    for row in 0..height {
        for col in 0..width {
            let offset = row * stride + col * 2;
            y.push(u16::from_le_bytes([y_data[offset], y_data[offset + 1]]));
        }
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

/// 16-bit フレームをエンコードしてパケットを返すヘルパー
///
/// 各プレーンはリトルエンディアンの u8 バイト列に変換して libaom に渡す。
pub(crate) fn encode_frames_16bit(
    config: EncoderConfig,
    frames: &[(Vec<u16>, Vec<u16>, Vec<u16>)],
) -> Vec<Vec<u8>> {
    let image_format = config.image_format;
    let mut encoder = Encoder::new(config).expect("failed to create encoder");
    let options = EncodeOptions {
        force_keyframe: false,
    };
    let mut encoded_packets = Vec::new();

    for (y, u, v) in frames {
        let y: Vec<u8> = y.iter().flat_map(|v| v.to_le_bytes()).collect();
        let u: Vec<u8> = u.iter().flat_map(|v| v.to_le_bytes()).collect();
        let v: Vec<u8> = v.iter().flat_map(|v| v.to_le_bytes()).collect();

        let image = match image_format {
            ImageFormat::I42016 => ImageData::I42016 {
                y: &y,
                u: &u,
                v: &v,
            },
            ImageFormat::I42216 => ImageData::I42216 {
                y: &y,
                u: &u,
                v: &v,
            },
            ImageFormat::I44416 => ImageData::I44416 {
                y: &y,
                u: &u,
                v: &v,
            },
            _ => panic!("encode_frames_16bit: unsupported format {image_format:?}"),
        };
        encoder.encode(&image, &options).expect("failed to encode");
        while let Some(encoded) = encoder.next_frame() {
            encoded_packets.push(encoded.data().expect("failed to get encoded data").to_vec());
        }
    }

    drain_after_finish(&mut encoder, &mut encoded_packets);

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

/// デコードして 16-bit Y プレーンとフォーマットの一覧を返すヘルパー
///
/// デコード結果のフォーマット検証に使えるよう、各フレームの `ImageFormat` も返す。
pub(crate) fn decode_frames_16bit(
    packets: &[Vec<u8>],
) -> Vec<(Vec<u16>, usize, usize, ImageFormat)> {
    let mut decoder = Decoder::new(DecoderConfig::default()).expect("failed to create decoder");
    let mut decoded = Vec::new();

    for packet in packets {
        decoder.decode(packet).expect("failed to decode");
        while let Some(frame) = decoder.next_frame() {
            let format = frame.format().expect("failed to get frame format");
            decoded.push((
                extract_y_plane_16bit(&frame),
                frame.width(),
                frame.height(),
                format,
            ));
        }
    }

    decoder.finish().expect("failed to finish");
    while let Some(frame) = decoder.next_frame() {
        let format = frame.format().expect("failed to get frame format");
        decoded.push((
            extract_y_plane_16bit(&frame),
            frame.width(),
            frame.height(),
            format,
        ));
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
/// 16-bit カラーバーを使ったラウンドトリップで PSNR とフォーマットを検証するヘルパー
///
/// 同一のカラーバーフレームを `num_frames` 回エンコードし、デコード後に
/// フォーマットが `config.image_format` と一致することと、Y プレーンの PSNR が
/// `min_psnr_db` 以上であることを確認する。
pub(crate) fn roundtrip_colorbar_16bit(config: EncoderConfig, num_frames: usize, min_psnr_db: f64) {
    let width = config.g_w as usize;
    let height = config.g_h as usize;
    let expected_format = config.image_format;
    let bit_depth = config
        .g_bit_depth
        .expect("g_bit_depth must be set for 16-bit encoding") as usize;

    let (y, u, v) = generate_colorbar_16bit(width, height, expected_format, bit_depth);
    let input_frames: Vec<(Vec<u16>, Vec<u16>, Vec<u16>)> = (0..num_frames)
        .map(|_| (y.clone(), u.clone(), v.clone()))
        .collect();

    let packets = encode_frames_16bit(config, &input_frames);
    assert!(!packets.is_empty(), "no encoded packets");

    let decoded_frames = decode_frames_16bit(&packets);
    assert_eq!(
        decoded_frames.len(),
        num_frames,
        "decoded {} frames, expected {num_frames}",
        decoded_frames.len()
    );
    for (i, (decoded_y, w, h, format)) in decoded_frames.iter().enumerate() {
        assert_eq!(*w, width, "decoded frame {i} width mismatch");
        assert_eq!(*h, height, "decoded frame {i} height mismatch");
        assert_eq!(
            *format, expected_format,
            "decoded frame {i} format mismatch"
        );
        let psnr = psnr_y_16bit(&y, decoded_y, width, height, bit_depth);
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

/// 16-bit フォーマット用のエンコーダー設定を生成するヘルパー
///
/// 16-bit フォーマットでは `g_bit_depth` が有効ビット深度を決め、
/// `g_profile` はフォーマットとの整合が検証される (詳細は README 参照)。
pub(crate) fn highbitdepth_config(
    width: u32,
    height: u32,
    format: ImageFormat,
    bit_depth: u32,
    profile: u32,
) -> EncoderConfig {
    let mut config = realtime_config(width, height, RateControlMode::Cbr);
    config.image_format = format;
    config.g_bit_depth = Some(bit_depth);
    config.g_profile = profile;
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
