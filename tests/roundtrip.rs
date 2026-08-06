use shiguredo_aom::{
    DecoderConfig, EncodeOptions, Encoder, EncoderConfig, ImageData, ImageFormat, KeyframeMode,
    RateControlMode, Usage,
};

mod helpers;
use helpers::*;

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
// 16-bit フォーマットテスト
// ============================================================================

/// I42016 + 10-bit + profile 0 のラウンドトリップ
///
/// デコード結果が 8-bit に落ちず I42016 のままであることも確認する。
#[test]
fn test_roundtrip_i42016_10bit() {
    let config = highbitdepth_config(320, 240, ImageFormat::I42016, 10, 0);
    roundtrip_colorbar_16bit(config, 10, 50.0);
}

/// I42216 + 10-bit + profile 2 のラウンドトリップ
#[test]
fn test_roundtrip_i42216_10bit() {
    let config = highbitdepth_config(320, 240, ImageFormat::I42216, 10, 2);
    roundtrip_colorbar_16bit(config, 10, 50.0);
}

/// I44416 + 10-bit + profile 1 のラウンドトリップ
#[test]
fn test_roundtrip_i44416_10bit() {
    let config = highbitdepth_config(320, 240, ImageFormat::I44416, 10, 1);
    roundtrip_colorbar_16bit(config, 10, 50.0);
}

/// I42016 + 12-bit + profile 2 のラウンドトリップ
#[test]
fn test_roundtrip_i42016_12bit() {
    let config = highbitdepth_config(320, 240, ImageFormat::I42016, 12, 2);
    roundtrip_colorbar_16bit(config, 10, 50.0);
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

/// realtime 配信向け 7 推奨値 + `KeyframeMode::Disabled` を `Usage::GoodQuality` ベースに
/// 上書きするヘルパー
///
/// shiguredo_aom がリンクする libaom (通常ビルド) では `Usage::Realtime` を選んでも
/// 7 フィールドのデフォルトは GoodQuality 寄り (`enable_*` 系 = 1、`*_cost_upd_freq`
/// = SB) のままで変わらない。本ヘルパーで明示的に realtime 推奨値を上書きすることで、
/// 7 フィールドが実際に Encoder へ反映される経路をテストできる。
fn good_quality_with_realtime_controls(width: u32, height: u32) -> EncoderConfig {
    let mut config = good_quality_config(width, height, RateControlMode::Cbr);
    config.enable_order_hint = Some(false);
    config.enable_ref_frame_mvs = Some(false);
    config.enable_angle_delta = Some(false);
    config.intra_default_tx_only = Some(true);
    config.coeff_cost_upd_freq = Some(3);
    config.mode_cost_upd_freq = Some(3);
    config.mv_cost_upd_freq = Some(3);
    config.kf_mode = Some(KeyframeMode::Disabled);
    config
}

/// `Usage::Realtime` ベースで 7 推奨値 + `KeyframeMode::Disabled` を指定するヘルパー
///
/// `Usage::Realtime` + `g_lag_in_frames = Some(0)` 下では `force_keyframe` が同フレームで
/// 反映されるため、フレーム単位の厳密 assert が可能。
fn realtime_with_disabled_keyframe(width: u32, height: u32) -> EncoderConfig {
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
    config
}

/// realtime 配信向けの典型セットを `Usage::GoodQuality` 上で適用してラウンドトリップする
/// (PSNR 検証)
///
/// `Usage::GoodQuality` ベースで 7 フィールドを realtime 推奨値に上書きすることで、
/// 7 フィールドが GoodQuality デフォルト (= 重い設定) と異なる経路に影響することを
/// 間接的に検証する。`Usage::Realtime` ベースでは libaom 通常ビルドの挙動として
/// 7 フィールドのデフォルトが既に同じ値ではないため、上書きしないと検証経路が
/// libaom デフォルトのままになってしまう。
#[test]
fn test_roundtrip_realtime_controls_typical_set() {
    let config = good_quality_with_realtime_controls(320, 240);
    roundtrip_colorbar(config, 30, 25.0);
}

/// `KeyframeMode::Disabled` 指定時に `kf_min_dist` / `kf_max_dist` のデフォルト経路で
/// 自動キーフレーム挿入が抑止されることを確認する
///
/// ここで検証するのは「`kf_max_dist` のデフォルト周期挿入が止まる」ことのみ。
/// シーンチェンジ抑制の本質検証 (Auto との対比) は別途行う必要がある。
#[test]
fn test_keyframe_mode_disabled_suppresses_periodic_keyframe() {
    let width = 320;
    let height = 240;
    let num_frames = 30;

    let config = realtime_with_disabled_keyframe(width, height);
    let mut encoder = Encoder::new(config).expect("failed to create encoder");
    let mut keyframe_flags = Vec::new();
    let mut packets = Vec::new();

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
        let pre = keyframe_flags.len();
        encoder.encode(&image, &options).expect("failed to encode");
        while let Some(encoded) = encoder.next_frame() {
            keyframe_flags.push(encoded.is_keyframe());
            packets.push(encoded.data().expect("failed to get encoded data").to_vec());
        }
        assert!(
            keyframe_flags.len() > pre,
            "frame {i}: encoder produced no output (g_lag_in_frames=0 expected to drain)"
        );
    }
    let pre_finish = keyframe_flags.len();
    encoder.finish().expect("failed to finish");
    while let Some(encoded) = encoder.next_frame() {
        keyframe_flags.push(encoded.is_keyframe());
        packets.push(encoded.data().expect("failed to get encoded data").to_vec());
    }
    assert_eq!(
        keyframe_flags.len(),
        pre_finish,
        "finish should not produce extra frames under g_lag_in_frames=0"
    );

    assert_eq!(
        keyframe_flags.len(),
        num_frames,
        "expected {num_frames} encoded frames, got {}",
        keyframe_flags.len()
    );
    assert!(
        keyframe_flags[0],
        "expected the first frame to be a keyframe (AV1 sequence header constraint)"
    );
    for (i, &is_key) in keyframe_flags.iter().enumerate().skip(1) {
        assert!(
            !is_key,
            "frame {i} should not be a keyframe under KeyframeMode::Disabled"
        );
    }

    // bitstream が実際にデコード可能であることまで確認する
    let decoded_frames = decode_frames(&packets);
    assert_eq!(decoded_frames.len(), num_frames);
}

/// `KeyframeMode::Disabled` 指定時でも `force_keyframe = true` でキーフレームを挿入できることを確認する
#[test]
fn test_keyframe_mode_disabled_with_force_keyframe() {
    let width = 320;
    let height = 240;
    let num_frames = 30;
    let force_index = 10;

    let config = realtime_with_disabled_keyframe(width, height);
    let mut encoder = Encoder::new(config).expect("failed to create encoder");
    let mut keyframe_flags = Vec::new();
    let mut packets = Vec::new();

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
            packets.push(encoded.data().expect("failed to get encoded data").to_vec());
        }
    }
    encoder.finish().expect("failed to finish");
    while let Some(encoded) = encoder.next_frame() {
        keyframe_flags.push(encoded.is_keyframe());
        packets.push(encoded.data().expect("failed to get encoded data").to_vec());
    }

    assert_eq!(
        keyframe_flags.len(),
        num_frames,
        "expected {num_frames} encoded frames, got {}",
        keyframe_flags.len()
    );
    for (i, &is_key) in keyframe_flags.iter().enumerate() {
        let expected = i == 0 || i == force_index;
        assert_eq!(is_key, expected, "frame {i}");
    }

    let decoded_frames = decode_frames(&packets);
    assert_eq!(decoded_frames.len(), num_frames);
}

/// `*_cost_upd_freq` の値域 (`0..=3`) を超える値を渡すと `Encoder::new` がエラーを返すことを確認する
///
/// libaom の `validate_config` で `RANGE_CHECK(extra_cfg, *_cost_upd_freq, 0, 3)` が
/// 走るため、aom-rs 側のフィールド ID と control ID の紐付けが正しければ確実にエラーになる。
#[test]
fn test_cost_upd_freq_out_of_range_returns_error() {
    type Setter = fn(&mut EncoderConfig);
    let cases: &[(&str, Setter)] = &[
        ("coeff_cost_upd_freq", |c| c.coeff_cost_upd_freq = Some(99)),
        ("mode_cost_upd_freq", |c| c.mode_cost_upd_freq = Some(99)),
        ("mv_cost_upd_freq", |c| c.mv_cost_upd_freq = Some(99)),
    ];

    for (name, set) in cases {
        let mut config = good_quality_config(320, 240, RateControlMode::Cbr);
        set(&mut config);
        let result = Encoder::new(config);
        assert!(
            result.is_err(),
            "{name} = Some(99) must be rejected by libaom RANGE_CHECK, but Encoder::new succeeded"
        );
    }
}

/// `KeyframeMode::Auto` 指定時に `kf_max_dist` を短く設定すると周期的にキーフレームが
/// 挿入されることを確認する
#[test]
fn test_keyframe_mode_auto_inserts_periodic_keyframe() {
    let width = 320;
    let height = 240;
    let num_frames = 30;

    let mut config = good_quality_config(width, height, RateControlMode::Cbr);
    config.kf_mode = Some(KeyframeMode::Auto);
    // kf_min_dist != kf_max_dist でないと libaom 側で auto_key が無効化される
    config.kf_min_dist = Some(0);
    config.kf_max_dist = Some(5);

    let mut encoder = Encoder::new(config).expect("failed to create encoder");
    let mut keyframe_count = 0;

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
            if encoded.is_keyframe() {
                keyframe_count += 1;
            }
            let _data = encoded.data().expect("failed to get encoded data");
        }
    }
    encoder.finish().expect("failed to finish");
    while let Some(encoded) = encoder.next_frame() {
        if encoded.is_keyframe() {
            keyframe_count += 1;
        }
        let _data = encoded.data().expect("failed to get encoded data");
    }

    // 30 フレーム / kf_max_dist=5 で先頭含め複数回挿入されるはず
    assert!(
        keyframe_count >= 2,
        "expected at least 2 keyframes under KeyframeMode::Auto with kf_max_dist=5, got {keyframe_count}"
    );
}

/// `KeyframeMode::Fixed` (deprecated alias) が `Disabled` と同一挙動になることを確認する
///
/// libaom 側で `AOM_KF_FIXED == AOM_KF_DISABLED == 0` であり、Rust 側 enum の
/// 整合性を回帰として固定する。
#[test]
#[expect(deprecated)]
fn test_keyframe_mode_fixed_behaves_like_disabled() {
    let width = 320;
    let height = 240;
    let num_frames = 15;

    let mut config = good_quality_config(width, height, RateControlMode::Cbr);
    config.kf_mode = Some(KeyframeMode::Fixed);

    let mut encoder = Encoder::new(config).expect("failed to create encoder");
    let mut keyframe_count = 0;

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
            if encoded.is_keyframe() {
                keyframe_count += 1;
            }
            let _data = encoded.data().expect("failed to get encoded data");
        }
    }
    encoder.finish().expect("failed to finish");
    while let Some(encoded) = encoder.next_frame() {
        if encoded.is_keyframe() {
            keyframe_count += 1;
        }
        let _data = encoded.data().expect("failed to get encoded data");
    }

    assert_eq!(
        keyframe_count, 1,
        "KeyframeMode::Fixed must behave like Disabled (only the first frame is a keyframe)"
    );
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
            panic!("encoding profiles should not be Unsupported");
        }
    }
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
        "force_keyframe = true did not produce a keyframe immediately after reconfigure"
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
