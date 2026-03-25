use shiguredo_aom::{
    Decoder, DecoderConfig, EncodeOptions, Encoder, EncoderConfig, ImageData, ImageFormat,
    RateControlMode, Usage,
};

// ============================================================================
// フレーム生成ヘルパー
// ============================================================================

/// ダミー I420 フレームを生成する
///
/// Y プレーンはフレーム番号に応じたグラデーション、UV プレーンは 128 固定。
fn generate_dummy_i420(
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
fn generate_colorbar_i420(width: usize, height: usize) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
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
fn psnr_y(original: &[u8], decoded: &[u8], width: usize, height: usize) -> f64 {
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
fn extract_y_plane(frame: &shiguredo_aom::DecodedFrame<'_>) -> Vec<u8> {
    let width = frame.width();
    let height = frame.height();
    let stride = frame.y_stride();
    let y_data = frame.y_plane();
    let mut y = Vec::with_capacity(width * height);
    for row in 0..height {
        y.extend_from_slice(&y_data[row * stride..row * stride + width]);
    }
    y
}

/// エンコードしてフレーム単位のビットストリームを返すヘルパー
///
/// AV1 デコーダーはフレーム単位でデータを受け取る必要があるため、
/// エンコード済みフレームごとに分割して返す。
fn encode_frames(config: EncoderConfig, frames: &[(Vec<u8>, Vec<u8>, Vec<u8>)]) -> Vec<Vec<u8>> {
    let mut encoder = Encoder::new(config).expect("failed to create encoder");
    let options = EncodeOptions {
        force_keyframe: false,
    };
    let mut encoded_packets = Vec::new();

    for (y, u, v) in frames {
        let image = ImageData::I420 { y, u, v };
        encoder.encode(&image, &options).expect("failed to encode");
        while let Some(encoded) = encoder.next_frame() {
            encoded_packets.push(encoded.data().to_vec());
        }
    }

    encoder.finish().expect("failed to finish");
    while let Some(encoded) = encoder.next_frame() {
        encoded_packets.push(encoded.data().to_vec());
    }

    encoded_packets
}

/// デコードしてフレーム情報一覧を返すヘルパー
///
/// デコード結果の Y プレーン、幅、高さを返す。
fn decode_frames(packets: &[Vec<u8>]) -> Vec<(Vec<u8>, usize, usize)> {
    decode_frames_with_config(DecoderConfig::default(), packets)
}

/// 指定したデコーダー設定でデコードしてフレーム情報一覧を返すヘルパー
fn decode_frames_with_config(
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
fn roundtrip(
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
fn roundtrip_colorbar(config: EncoderConfig, num_frames: usize, min_psnr_db: f64) {
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
fn realtime_config(width: u32, height: u32, rate_control: RateControlMode) -> EncoderConfig {
    let mut config = EncoderConfig::new(width, height, ImageFormat::I420);
    config.g_usage = Usage::Realtime;
    config.rc_end_usage = rate_control;
    config.rc_target_bitrate = 1000;
    config.cpu_used = Some(8);
    config
}

/// GoodQuality モードのエンコーダー設定を生成するヘルパー
fn good_quality_config(width: u32, height: u32, rate_control: RateControlMode) -> EncoderConfig {
    let mut config = EncoderConfig::new(width, height, ImageFormat::I420);
    config.g_usage = Usage::GoodQuality;
    config.rc_end_usage = rate_control;
    config.rc_target_bitrate = 1000;
    config.cpu_used = Some(8);
    config.g_lag_in_frames = Some(0);
    config
}

/// AllIntra モードのエンコーダー設定を生成するヘルパー
fn all_intra_config(width: u32, height: u32, rate_control: RateControlMode) -> EncoderConfig {
    let mut config = EncoderConfig::new(width, height, ImageFormat::I420);
    config.g_usage = Usage::AllIntra;
    config.rc_end_usage = rate_control;
    config.rc_target_bitrate = 1000;
    config.cpu_used = Some(8);
    config
}

// ============================================================================
// Realtime モードテスト
// ============================================================================

/// Realtime CBR ダミーフレームのラウンドトリップ
#[test]
fn test_roundtrip_realtime_cbr() {
    let width = 320;
    let height = 240;
    let num_frames = 10;

    let config = realtime_config(width, height, RateControlMode::Cbr);

    let input_frames: Vec<(Vec<u8>, Vec<u8>, Vec<u8>)> = (0..num_frames)
        .map(|i| generate_dummy_i420(width as usize, height as usize, i))
        .collect();

    let decoded_frames = roundtrip(config, &input_frames);
    assert_eq!(decoded_frames.len(), num_frames);
}

/// Realtime VBR ダミーフレームのラウンドトリップ
#[test]
fn test_roundtrip_realtime_vbr() {
    let width = 320;
    let height = 240;
    let num_frames = 10;

    let config = realtime_config(width, height, RateControlMode::Vbr);

    let input_frames: Vec<(Vec<u8>, Vec<u8>, Vec<u8>)> = (0..num_frames)
        .map(|i| generate_dummy_i420(width as usize, height as usize, i))
        .collect();

    let decoded_frames = roundtrip(config, &input_frames);
    assert_eq!(decoded_frames.len(), num_frames);
}

/// Realtime CBR カラーバーのラウンドトリップ（PSNR 検証）
#[test]
fn test_psnr_realtime_cbr() {
    let config = realtime_config(320, 240, RateControlMode::Cbr);
    roundtrip_colorbar(config, 30, 25.0);
}

/// Realtime VBR カラーバーのラウンドトリップ（PSNR 検証）
#[test]
fn test_psnr_realtime_vbr() {
    let config = realtime_config(320, 240, RateControlMode::Vbr);
    roundtrip_colorbar(config, 10, 25.0);
}

// ============================================================================
// GoodQuality モードテスト
// ============================================================================

/// GoodQuality CBR カラーバーのラウンドトリップ（PSNR 検証）
#[test]
fn test_psnr_good_quality_cbr() {
    let config = good_quality_config(320, 240, RateControlMode::Cbr);
    roundtrip_colorbar(config, 10, 25.0);
}

/// GoodQuality VBR カラーバーのラウンドトリップ（PSNR 検証）
#[test]
fn test_psnr_good_quality_vbr() {
    let config = good_quality_config(320, 240, RateControlMode::Vbr);
    roundtrip_colorbar(config, 10, 25.0);
}

/// GoodQuality CQ カラーバーのラウンドトリップ（PSNR 検証）
#[test]
fn test_psnr_good_quality_cq() {
    let mut config = good_quality_config(320, 240, RateControlMode::Cq);
    config.cq_level = Some(30);
    roundtrip_colorbar(config, 10, 25.0);
}

/// GoodQuality Q カラーバーのラウンドトリップ（PSNR 検証）
#[test]
fn test_psnr_good_quality_q() {
    let mut config = good_quality_config(320, 240, RateControlMode::Q);
    config.cq_level = Some(30);
    roundtrip_colorbar(config, 10, 25.0);
}

// ============================================================================
// AllIntra モードテスト
// ============================================================================

/// AllIntra Q カラーバーのラウンドトリップ（PSNR 検証）
#[test]
fn test_psnr_all_intra_q() {
    let mut config = all_intra_config(320, 240, RateControlMode::Q);
    config.cq_level = Some(30);
    roundtrip_colorbar(config, 5, 25.0);
}

// ============================================================================
// キーフレーム強制テスト
// ============================================================================

/// Realtime でキーフレームを強制してラウンドトリップする
#[test]
fn test_roundtrip_force_keyframe() {
    let width = 320;
    let height = 240;

    let config = realtime_config(width, height, RateControlMode::Cbr);

    let mut encoder = Encoder::new(config).expect("failed to create encoder");
    let mut packets = Vec::new();
    let mut keyframe_count = 0;

    for i in 0..15 {
        let (y, u, v) = generate_dummy_i420(width as usize, height as usize, i);
        let options = EncodeOptions {
            force_keyframe: i == 10,
        };
        let image = ImageData::I420 {
            y: &y,
            u: &u,
            v: &v,
        };
        encoder.encode(&image, &options).expect("failed to encode");
        while let Some(encoded) = encoder.next_frame() {
            if encoded.is_keyframe() {
                keyframe_count += 1;
            }
            packets.push(encoded.data().to_vec());
        }
    }
    encoder.finish().expect("failed to finish");
    while let Some(encoded) = encoder.next_frame() {
        if encoded.is_keyframe() {
            keyframe_count += 1;
        }
        packets.push(encoded.data().to_vec());
    }

    assert!(
        keyframe_count >= 2,
        "expected at least 2 keyframes, got {keyframe_count}"
    );

    // デコードで復号できることを確認する
    let decoded_frames = decode_frames(&packets);
    assert_eq!(decoded_frames.len(), 15);
}

// ============================================================================
// 解像度別テスト
// ============================================================================

/// 640x480 Realtime CBR カラーバーのラウンドトリップ（PSNR 検証）
#[test]
fn test_psnr_realtime_cbr_640x480() {
    let mut config = realtime_config(640, 480, RateControlMode::Cbr);
    config.rc_target_bitrate = 2000;
    roundtrip_colorbar(config, 10, 25.0);
}

/// 160x120 Realtime CBR カラーバーのラウンドトリップ（PSNR 検証）
#[test]
fn test_psnr_realtime_cbr_160x120() {
    let config = realtime_config(160, 120, RateControlMode::Cbr);
    roundtrip_colorbar(config, 10, 20.0);
}

// ============================================================================
// デコーダー設定テスト
// ============================================================================

/// デコーダーにスレッド数を指定してラウンドトリップする
#[test]
fn test_roundtrip_decoder_threads() {
    let config = realtime_config(320, 240, RateControlMode::Cbr);
    let num_frames = 10;

    let input_frames: Vec<(Vec<u8>, Vec<u8>, Vec<u8>)> = (0..num_frames)
        .map(|i| generate_dummy_i420(320, 240, i))
        .collect();

    let packets = encode_frames(config, &input_frames);
    assert!(!packets.is_empty(), "no encoded packets");

    let mut dec_config = DecoderConfig::new();
    dec_config.threads = Some(2);

    let decoded_frames = decode_frames_with_config(dec_config, &packets);
    assert_eq!(decoded_frames.len(), num_frames);
    for (i, (y, w, h)) in decoded_frames.iter().enumerate() {
        assert_eq!(*w, 320, "decoded frame {i} width mismatch");
        assert_eq!(*h, 240, "decoded frame {i} height mismatch");
        assert!(!y.is_empty(), "decoded frame {i} has empty Y plane");
    }
}

// ============================================================================
// エンコーダーオプションテスト
// ============================================================================

/// row_mt 有効で Realtime CBR カラーバーのラウンドトリップ（PSNR 検証）
#[test]
fn test_psnr_realtime_cbr_row_mt() {
    let mut config = realtime_config(320, 240, RateControlMode::Cbr);
    config.row_mt = Some(true);
    config.tile_columns = Some(1);
    config.tile_rows = Some(1);
    roundtrip_colorbar(config, 10, 25.0);
}

/// ロスレスモードのラウンドトリップ
#[test]
fn test_roundtrip_lossless() {
    let width: u32 = 64;
    let height: u32 = 64;

    let mut config = EncoderConfig::new(width, height, ImageFormat::I420);
    config.g_usage = Usage::AllIntra;
    config.rc_end_usage = RateControlMode::Q;
    config.cpu_used = Some(8);
    config.lossless = Some(true);

    let (y, u, v) = generate_colorbar_i420(width as usize, height as usize);
    let input_frames = vec![(y.clone(), u.clone(), v.clone())];

    let decoded_frames = roundtrip(config, &input_frames);

    // ロスレスなので PSNR は無限大（完全一致）
    let psnr = psnr_y(&y, &decoded_frames[0].0, width as usize, height as usize);
    assert!(
        psnr == f64::INFINITY,
        "lossless mode: PSNR {psnr:.1} dB, expected INFINITY"
    );
}

// ============================================================================
// supported_codecs テスト
// ============================================================================

#[test]
fn test_supported_codecs() {
    let info = shiguredo_aom::supported_codecs();

    // コーデック種別
    assert_eq!(info.codec, shiguredo_aom::VideoCodecType::Av1);

    // デコード対応
    assert!(info.decoding.supported);
    assert!(!info.decoding.hardware_accelerated);

    // エンコード対応
    assert!(info.encoding.supported);
    assert!(!info.encoding.hardware_accelerated);

    // Profile 0 は必ず対応している
    match &info.encoding.profiles {
        shiguredo_aom::EncodingProfiles::Av1(profiles) => {
            assert!(profiles.contains(&shiguredo_aom::Av1EncodingProfile::Profile0));
        }
        shiguredo_aom::EncodingProfiles::Unsupported => {
            panic!("エンコードプロファイルが Unsupported になっている");
        }
    }
}
