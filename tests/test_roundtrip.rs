use shiguredo_aom::{
    Decoder, DecoderConfig, EncodeOptions, Encoder, EncoderConfig, ImageData, ImageFormat,
    KeyframeMode, RateControlMode, ReconfigureParams, Usage,
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
    let stride = frame.y_stride().expect("failed to get Y stride");
    let y_data = frame.y_plane().expect("failed to get Y plane");
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
            packets.push(encoded.data().expect("failed to get encoded data").to_vec());
        }
    }
    encoder.finish().expect("failed to finish");
    while let Some(encoded) = encoder.next_frame() {
        if encoded.is_keyframe() {
            keyframe_count += 1;
        }
        packets.push(encoded.data().expect("failed to get encoded data").to_vec());
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
// realtime 制御フラグテスト
// ============================================================================

/// realtime 配信向けの典型セットでカラーバーをラウンドトリップする (PSNR 検証)
#[test]
fn test_roundtrip_realtime_controls_typical_set() {
    let mut config = realtime_config(320, 240, RateControlMode::Cbr);
    config.enable_order_hint = Some(false);
    config.enable_ref_frame_mvs = Some(false);
    config.enable_angle_delta = Some(false);
    config.intra_default_tx_only = Some(true);
    config.coeff_cost_upd_freq = Some(3);
    config.mode_cost_upd_freq = Some(3);
    config.mv_cost_upd_freq = Some(3);
    config.kf_mode = Some(KeyframeMode::Disabled);

    roundtrip_colorbar(config, 30, 25.0);
}

/// `KeyframeMode::Disabled` 指定時に自動キーフレーム挿入が停止することを確認する
#[test]
fn test_keyframe_mode_disabled_suppresses_auto_keyframe() {
    let width = 320;
    let height = 240;
    let num_frames = 30;

    let mut config = realtime_config(width, height, RateControlMode::Cbr);
    config.enable_order_hint = Some(false);
    config.enable_ref_frame_mvs = Some(false);
    config.enable_angle_delta = Some(false);
    config.intra_default_tx_only = Some(true);
    config.coeff_cost_upd_freq = Some(3);
    config.mode_cost_upd_freq = Some(3);
    config.mv_cost_upd_freq = Some(3);
    config.kf_mode = Some(KeyframeMode::Disabled);
    config.g_lag_in_frames = Some(0);

    let mut encoder = Encoder::new(config).expect("failed to create encoder");
    let mut keyframe_flags = Vec::new();

    for i in 0..num_frames {
        let (y, u, v) = generate_dummy_i420(width as usize, height as usize, i);
        let options = EncodeOptions {
            force_keyframe: false,
        };
        let image = ImageData::I420 {
            y: &y,
            u: &u,
            v: &v,
        };
        encoder.encode(&image, &options).expect("failed to encode");
        while let Some(encoded) = encoder.next_frame() {
            keyframe_flags.push(encoded.is_keyframe());
            let _ = encoded.data().expect("failed to get encoded data");
        }
    }
    encoder.finish().expect("failed to finish");
    while let Some(encoded) = encoder.next_frame() {
        keyframe_flags.push(encoded.is_keyframe());
        let _ = encoded.data().expect("failed to get encoded data");
    }

    assert_eq!(
        keyframe_flags.len(),
        num_frames,
        "expected {num_frames} encoded frames, got {}",
        keyframe_flags.len()
    );
    assert!(
        keyframe_flags[0],
        "expected the first frame to be a keyframe"
    );
    for (i, &is_key) in keyframe_flags.iter().enumerate().skip(1) {
        assert!(
            !is_key,
            "expected frame {i} to be a non-keyframe under KeyframeMode::Disabled"
        );
    }
}

/// `KeyframeMode::Disabled` 指定時でも `force_keyframe = true` でキーフレームを挿入できることを確認する
#[test]
fn test_keyframe_mode_disabled_with_force_keyframe() {
    let width = 320;
    let height = 240;
    let num_frames = 30;
    let force_index = 10;

    let mut config = realtime_config(width, height, RateControlMode::Cbr);
    config.enable_order_hint = Some(false);
    config.enable_ref_frame_mvs = Some(false);
    config.enable_angle_delta = Some(false);
    config.intra_default_tx_only = Some(true);
    config.coeff_cost_upd_freq = Some(3);
    config.mode_cost_upd_freq = Some(3);
    config.mv_cost_upd_freq = Some(3);
    config.kf_mode = Some(KeyframeMode::Disabled);
    config.g_lag_in_frames = Some(0);

    let mut encoder = Encoder::new(config).expect("failed to create encoder");
    let mut keyframe_flags = Vec::new();

    for i in 0..num_frames {
        let (y, u, v) = generate_dummy_i420(width as usize, height as usize, i);
        let options = EncodeOptions {
            force_keyframe: i == force_index,
        };
        let image = ImageData::I420 {
            y: &y,
            u: &u,
            v: &v,
        };
        encoder.encode(&image, &options).expect("failed to encode");
        while let Some(encoded) = encoder.next_frame() {
            keyframe_flags.push(encoded.is_keyframe());
            let _ = encoded.data().expect("failed to get encoded data");
        }
    }
    encoder.finish().expect("failed to finish");
    while let Some(encoded) = encoder.next_frame() {
        keyframe_flags.push(encoded.is_keyframe());
        let _ = encoded.data().expect("failed to get encoded data");
    }

    assert_eq!(
        keyframe_flags.len(),
        num_frames,
        "expected {num_frames} encoded frames, got {}",
        keyframe_flags.len()
    );
    for (i, &is_key) in keyframe_flags.iter().enumerate() {
        let expected = i == 0 || i == force_index;
        assert_eq!(
            is_key, expected,
            "frame {i}: expected is_keyframe={expected}, got {is_key}"
        );
    }
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

// ============================================================================
// reconfigure テスト用ヘルパー
// ============================================================================

/// libaom の `rc_target_bitrate` 受け入れ上限 (kbps)
///
/// 出典: 生成済み bindings の doc コメント (`Max allowed value is 2000000`)
const LIBAOM_RC_TARGET_BITRATE_MAX_KBPS: u32 = 2_000_000;

/// `rc_target_bitrate` だけを変更する [`ReconfigureParams`] を生成するヘルパー
fn rc_bitrate(kbps: u32) -> ReconfigureParams {
    ReconfigureParams {
        rc_target_bitrate: Some(kbps),
    }
}

/// ダミー I420 フレームをエンコードしてパケットを `packets` に積む
fn drive_dummy(
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
fn drain_after_finish(encoder: &mut Encoder, packets: &mut Vec<Vec<u8>>) {
    encoder.finish().expect("failed to finish");
    while let Some(encoded) = encoder.next_frame() {
        packets.push(encoded.data().expect("failed to get encoded data").to_vec());
    }
}

fn total_bytes(packets: &[Vec<u8>]) -> u64 {
    packets.iter().map(|p| p.len() as u64).sum()
}

// ============================================================================
// reconfigure テスト
// ============================================================================

/// エンコード途中で `reconfigure` を呼んでビットレートを大幅に下げ、
/// 後半合計バイト数が前半より明確に小さくなる (= 設定変更が実際に効いている)
/// ことを確認する
#[test]
fn test_reconfigure_target_bitrate_midstream() {
    let width = 320;
    let height = 240;
    let half = 12;

    // 高ビットレートから低ビットレートに切り替える方向で検証する。
    // 前半のキーフレーム膨張を考慮しても後半 (P フレーム主体・低ビットレート) が
    // 明確に小さくなるため、緩い閾値でも安定して検出できる。
    let mut config = realtime_config(width, height, RateControlMode::Cbr);
    config.rc_target_bitrate = 5000;

    let mut encoder = Encoder::new(config).expect("failed to create encoder");
    let options = EncodeOptions {
        force_keyframe: false,
    };
    let mut packets: Vec<Vec<u8>> = Vec::new();

    drive_dummy(&mut encoder, &options, width, height, 0..half, &mut packets);
    let before_count = packets.len();

    encoder
        .reconfigure(rc_bitrate(200))
        .expect("failed to reconfigure");

    drive_dummy(
        &mut encoder,
        &options,
        width,
        height,
        half..half * 2,
        &mut packets,
    );
    drain_after_finish(&mut encoder, &mut packets);

    let (before, after) = packets.split_at(before_count);
    let before_bytes = total_bytes(before);
    let after_bytes = total_bytes(after);
    assert!(
        after_bytes * 2 < before_bytes,
        "expected after_bytes * 2 < before_bytes (bitrate reconfigure had no effect): before={before_bytes}, after={after_bytes}",
    );
    assert_eq!(decode_frames(&packets).len(), half * 2);
}

/// `Encoder::new` 直後に reconfigure を呼び、そのままエンコード・デコードまで
/// 完走することを確認する
#[test]
fn test_reconfigure_immediately_after_new() {
    let width = 320;
    let height = 240;
    let num_frames = 4;
    let config = realtime_config(width, height, RateControlMode::Cbr);
    let mut encoder = Encoder::new(config).expect("failed to create encoder");

    encoder
        .reconfigure(rc_bitrate(500))
        .expect("failed to reconfigure");

    let options = EncodeOptions {
        force_keyframe: false,
    };
    let mut packets: Vec<Vec<u8>> = Vec::new();
    drive_dummy(
        &mut encoder,
        &options,
        width,
        height,
        0..num_frames,
        &mut packets,
    );
    drain_after_finish(&mut encoder, &mut packets);

    assert_eq!(decode_frames(&packets).len(), num_frames);
}

/// `next_frame()` がまだ完了していない (iter が非 NULL) 状態で reconfigure を
/// 呼ぶとエラーになることを確認する
#[test]
fn test_reconfigure_while_iter_active() {
    let width = 320;
    let height = 240;
    let config = realtime_config(width, height, RateControlMode::Cbr);
    let mut encoder = Encoder::new(config).expect("failed to create encoder");

    // realtime モードでは最初の encode から 1 フレーム以上出力されるため、
    // force_keyframe は不要
    let options = EncodeOptions {
        force_keyframe: false,
    };
    let (y, u, v) = generate_dummy_i420(width as usize, height as usize, 0);
    let image = ImageData::I420 {
        y: &y,
        u: &u,
        v: &v,
    };
    encoder.encode(&image, &options).expect("failed to encode");

    // 1 フレームだけ取り出して iter を非 NULL 状態にする
    let _ = encoder
        .next_frame()
        .expect("expected at least one encoded frame")
        .data()
        .expect("failed to get encoded data")
        .to_vec();

    let err = encoder
        .reconfigure(rc_bitrate(2000))
        .expect_err("reconfigure should fail while iter is active");
    assert!(
        err.to_string()
            .starts_with("shiguredo_aom::Encoder::reconfigure()"),
        "unexpected error message: {err}"
    );
}

/// `finish()` 後に `next_frame()` がまだ完了していない状態で reconfigure を
/// 呼ぶとエラーになることを確認する
///
/// realtime モードではエンコード時にフレームが即時出力されるため、`finish()` 後に
/// 出力が残らない。`g_lag_in_frames > 0` を指定できる GoodQuality モードでフレームを
/// 蓄積させてから `finish()` を呼び、残りフレームを 1 つだけ取り出して iter を
/// 非 NULL 状態にする。
#[test]
fn test_reconfigure_after_finish_while_iter_active() {
    let width = 320;
    let height = 240;
    let mut config = good_quality_config(width, height, RateControlMode::Vbr);
    config.g_lag_in_frames = Some(4);
    let mut encoder = Encoder::new(config).expect("failed to create encoder");
    let options = EncodeOptions {
        force_keyframe: false,
    };

    let mut packets = Vec::new();
    drive_dummy(&mut encoder, &options, width, height, 0..6, &mut packets);

    encoder.finish().expect("failed to finish");

    // finish 後の残りフレームを 1 つだけ取り出して iter を非 NULL 状態にする
    let _ = encoder
        .next_frame()
        .expect("expected at least one encoded frame after finish")
        .data()
        .expect("failed to get encoded data")
        .to_vec();

    let err = encoder
        .reconfigure(rc_bitrate(2000))
        .expect_err("reconfigure should fail while iter is active after finish");
    assert!(
        err.to_string()
            .starts_with("shiguredo_aom::Encoder::reconfigure()"),
        "unexpected error message: {err}"
    );
}

/// ビットレートを連続して複数回切り替えても、エンコード・デコードが
/// 完走することを確認する
#[test]
fn test_reconfigure_target_bitrate_multi_switch() {
    let width = 320;
    let height = 240;
    let bitrates = [1000u32, 2000, 500, 1500];
    let frames_per_segment = 3;
    let num_frames = bitrates.len() * frames_per_segment;

    let config = realtime_config(width, height, RateControlMode::Cbr);
    let mut encoder = Encoder::new(config).expect("failed to create encoder");
    let options = EncodeOptions {
        force_keyframe: false,
    };
    let mut packets: Vec<Vec<u8>> = Vec::new();

    for (segment, &bitrate) in bitrates.iter().enumerate() {
        encoder
            .reconfigure(rc_bitrate(bitrate))
            .expect("failed to reconfigure");

        let begin = segment * frames_per_segment;
        drive_dummy(
            &mut encoder,
            &options,
            width,
            height,
            begin..begin + frames_per_segment,
            &mut packets,
        );
    }
    drain_after_finish(&mut encoder, &mut packets);

    assert_eq!(decode_frames(&packets).len(), num_frames);
}

/// VBR モードでもエンコード途中の reconfigure が完走することを確認する
///
/// VBR は CBR ほどタイトに target に追従しないため、本テストでは合計バイト数の
/// 大小比較ではなく「reconfigure 後もエンコード・デコードが完走する」ことだけを
/// 確認する。ビットレート反映の検証は `test_reconfigure_target_bitrate_midstream`
/// (CBR) で担当する。
#[test]
fn test_reconfigure_target_bitrate_vbr() {
    let width = 320;
    let height = 240;
    let num_frames = 12;

    let config = realtime_config(width, height, RateControlMode::Vbr);
    let mut encoder = Encoder::new(config).expect("failed to create encoder");
    let options = EncodeOptions {
        force_keyframe: false,
    };
    let mut packets: Vec<Vec<u8>> = Vec::new();

    let half = num_frames / 2;
    drive_dummy(&mut encoder, &options, width, height, 0..half, &mut packets);
    encoder
        .reconfigure(rc_bitrate(2000))
        .expect("failed to reconfigure");
    drive_dummy(
        &mut encoder,
        &options,
        width,
        height,
        half..num_frames,
        &mut packets,
    );
    drain_after_finish(&mut encoder, &mut packets);

    assert_eq!(decode_frames(&packets).len(), num_frames);
}

/// reconfigure が libaom 検証で失敗した場合に、内部 `cfg` が変更前のまま保たれ、
/// 同じ範囲外値での再呼び出しでも同じく失敗することを確認する
///
/// `LIBAOM_RC_TARGET_BITRATE_MAX_KBPS` を超える値を渡して失敗を発生させる。
#[test]
fn test_reconfigure_state_unchanged_on_failure() {
    let width = 320;
    let height = 240;
    let config = realtime_config(width, height, RateControlMode::Cbr);
    let mut encoder = Encoder::new(config).expect("failed to create encoder");
    let options = EncodeOptions {
        force_keyframe: false,
    };

    let oor_bitrate = LIBAOM_RC_TARGET_BITRATE_MAX_KBPS + 1;

    let err = encoder
        .reconfigure(rc_bitrate(oor_bitrate))
        .expect_err("expected reconfigure to fail with out-of-range bitrate");
    assert!(
        err.to_string().starts_with("aom_codec_enc_config_set()"),
        "unexpected error message: {err}"
    );

    // 同じ範囲外値を再度渡しても同様に失敗する (= self.cfg が壊れていない)
    let err_retry = encoder
        .reconfigure(rc_bitrate(oor_bitrate))
        .expect_err("expected the second out-of-range reconfigure to also fail");
    assert!(
        err_retry
            .to_string()
            .starts_with("aom_codec_enc_config_set()"),
        "unexpected error message: {err_retry}"
    );

    // 妥当な値での reconfigure が成功し、エンコード・デコードまで完走する
    encoder
        .reconfigure(rc_bitrate(1500))
        .expect("reconfigure with valid bitrate should succeed after a failed call");

    let mut packets: Vec<Vec<u8>> = Vec::new();
    drive_dummy(&mut encoder, &options, width, height, 0..3, &mut packets);
    drain_after_finish(&mut encoder, &mut packets);
    assert_eq!(decode_frames(&packets).len(), 3);
}

/// reconfigure 直後に `force_keyframe: true` でエンコードしても完走することを確認する
///
/// 帯域急変時の典型パターン (ビットレートを上げると同時にキーフレームを打ち直す) を
/// 想定したテスト。
#[test]
fn test_reconfigure_followed_by_forced_keyframe() {
    let width = 320;
    let height = 240;
    let config = realtime_config(width, height, RateControlMode::Cbr);
    let mut encoder = Encoder::new(config).expect("failed to create encoder");
    let options_p = EncodeOptions {
        force_keyframe: false,
    };
    let options_key = EncodeOptions {
        force_keyframe: true,
    };

    let mut packets: Vec<Vec<u8>> = Vec::new();
    drive_dummy(&mut encoder, &options_p, width, height, 0..3, &mut packets);

    encoder
        .reconfigure(rc_bitrate(2000))
        .expect("failed to reconfigure");

    // reconfigure 直後にキーフレームを強制する
    let (y, u, v) = generate_dummy_i420(width as usize, height as usize, 3);
    let image = ImageData::I420 {
        y: &y,
        u: &u,
        v: &v,
    };
    encoder
        .encode(&image, &options_key)
        .expect("failed to encode keyframe after reconfigure");
    let mut keyframe_seen = false;
    while let Some(encoded) = encoder.next_frame() {
        if encoded.is_keyframe() {
            keyframe_seen = true;
        }
        packets.push(encoded.data().expect("failed to get encoded data").to_vec());
    }
    assert!(
        keyframe_seen,
        "force_keyframe = true で reconfigure 直後のフレームがキーフレームとして出力されなかった"
    );

    drive_dummy(&mut encoder, &options_p, width, height, 4..6, &mut packets);
    drain_after_finish(&mut encoder, &mut packets);

    assert_eq!(decode_frames(&packets).len(), 6);
}

/// reconfigure をはさんでもデコード後のフレームが元入力と整合する程度の画質を
/// 保つことを PSNR で確認する
///
/// カラーバー入力を使い、midstream で reconfigure (5000 → 1500 kbps) した後でも
/// 全フレームで PSNR が 20 dB 以上であることを確認する。
#[test]
fn test_reconfigure_psnr_midstream() {
    let width = 320usize;
    let height = 240usize;
    let num_frames = 16;

    let mut config = realtime_config(width as u32, height as u32, RateControlMode::Cbr);
    config.rc_target_bitrate = 5000;

    let (input_y, input_u, input_v) = generate_colorbar_i420(width, height);

    let mut encoder = Encoder::new(config).expect("failed to create encoder");
    let options = EncodeOptions {
        force_keyframe: false,
    };
    let mut packets: Vec<Vec<u8>> = Vec::new();

    for i in 0..num_frames {
        if i == num_frames / 2 {
            encoder
                .reconfigure(rc_bitrate(1500))
                .expect("failed to reconfigure");
        }
        let image = ImageData::I420 {
            y: &input_y,
            u: &input_u,
            v: &input_v,
        };
        encoder.encode(&image, &options).expect("failed to encode");
        while let Some(encoded) = encoder.next_frame() {
            packets.push(encoded.data().expect("failed to get encoded data").to_vec());
        }
    }
    drain_after_finish(&mut encoder, &mut packets);

    let decoded = decode_frames(&packets);
    assert_eq!(decoded.len(), num_frames);
    for (i, (y, _, _)) in decoded.iter().enumerate() {
        let psnr = psnr_y(&input_y, y, width, height);
        assert!(psnr >= 20.0, "frame {i}: PSNR {psnr:.1} dB < 20.0 dB");
    }
}
