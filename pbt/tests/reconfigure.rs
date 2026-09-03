//! reconfigure の性質を検証する PBT
//!
//! 対応する `src/` モジュールは無いため、ファイル名に `prop_` プレフィックスは付けない。

use std::cell::Cell;

use shiguredo_aom::{
    EncodeOptions, Encoder, EncoderConfig, ImageData, ImageFormat, RateControlMode,
    ReconfigureParams, Usage,
};

// PBT の 1 ケースあたりの実行数
//
// エンコード 1 回あたり数十 ms かかるため、デフォルトの 256 件ではなく
// 境界値を明示的に含めた上で件数を抑える。
const CASES: usize = 32;

// ビットレートの生成範囲 (kbps)
//
// proptest 時代の `200u32..5000` と同じ半開区間。
const BITRATE_RANGE: std::ops::Range<usize> = 200..5000;

// フレーム数の生成範囲
const SWITCH_AT_RANGE: std::ops::Range<usize> = 1..6;
const TRAILING_RANGE: std::ops::Range<usize> = 1..6;
const REPEATS_RANGE: std::ops::Range<usize> = 1..4;

/// ダミー I420 フレームを生成する
///
/// Y プレーンはフレーム番号に応じたグラデーション、UV プレーンは 128 固定。
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

/// Realtime CBR 設定を生成する
fn realtime_cbr(width: u32, height: u32, bitrate: u32) -> EncoderConfig {
    let mut config = EncoderConfig::new(width, height, ImageFormat::I420);
    config.g_usage = Usage::Realtime;
    config.rc_end_usage = RateControlMode::Cbr;
    config.rc_target_bitrate = bitrate;
    config.cpu_used = Some(8);
    config
}

/// ビットレートを境界値込みで生成する
///
/// 200 (下限) と 4999 (上限) に意味のある確率を与える。
fn sample_bitrate(ctx: &mut noprop::TestCaseContext) -> u32 {
    noprop::sample_with_boundaries(ctx, &[200usize, 4999], noprop::Ratio::one_nth(5), |ctx| {
        noprop::sample_usize_in(ctx, BITRATE_RANGE)
    }) as u32
}

/// 指定フレーム数をエンコードして出力数を返す
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
            .expect("エンコードに失敗した");
        while let Some(encoded) = encoder.next_frame() {
            let _ = encoded.data().expect("エンコードデータの取得に失敗した");
            count += 1;
        }
    }
    encoder.finish().expect("終了処理に失敗した");
    while let Some(encoded) = encoder.next_frame() {
        let _ = encoded.data().expect("エンコードデータの取得に失敗した");
        count += 1;
    }
    count
}

/// 任意の妥当なビットレートで midstream reconfigure してもエンコードが完走し、
/// 入力フレーム数ぶんの出力が得られること
#[test]
fn reconfigure_preserves_frame_count() -> noprop::TestResult {
    let seed = noprop::seed_from_env_or_time("SHIGUREDO_AOM_PBT_SEED")?;
    // ビットレート変更が実際に行われたケース数を数える
    //
    // 初期値と変更後が同値ばかりだと「変更しても完走する」性質を検証したことに
    // ならないため、空振り防止のゲートとして使う。
    let changed = Cell::new(0usize);
    let mut runner = noprop::Runner::new(seed);

    runner.run(CASES, |ctx| {
        let initial_bitrate = sample_bitrate(ctx);
        let new_bitrate = sample_bitrate(ctx);
        let switch_at = noprop::sample_usize_in(ctx, SWITCH_AT_RANGE);
        let trailing = noprop::sample_usize_in(ctx, TRAILING_RANGE);

        let width = 160u32;
        let height = 120u32;
        let mut encoder = Encoder::new(realtime_cbr(width, height, initial_bitrate))
            .expect("エンコーダーの生成に失敗した");

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
                .expect("再設定前のエンコードに失敗した");
            while let Some(encoded) = encoder.next_frame() {
                let _ = encoded.data().expect("エンコードデータの取得に失敗した");
                produced += 1;
            }
        }

        encoder
            .reconfigure(ReconfigureParams {
                rc_target_bitrate: Some(new_bitrate),
            })
            .expect("範囲内ビットレートでの再設定に失敗した");

        // 不変条件の評価地点でゲートを更新する
        //
        // 棄却ケースの混入を避けるため、このクロージャーでは reject を使わず、
        // ゲート更新より後に早期 return しない。
        if initial_bitrate != new_bitrate {
            changed.set(changed.get() + 1);
        }

        for i in 0..trailing {
            let (y, u, v) = generate_dummy_i420(width as usize, height as usize, switch_at + i);
            let image = ImageData::I420 {
                y: &y,
                u: &u,
                v: &v,
            };
            encoder
                .encode(&image, &options)
                .expect("再設定後のエンコードに失敗した");
            while let Some(encoded) = encoder.next_frame() {
                let _ = encoded.data().expect("エンコードデータの取得に失敗した");
                produced += 1;
            }
        }

        encoder.finish().expect("終了処理に失敗した");
        while let Some(encoded) = encoder.next_frame() {
            let _ = encoded.data().expect("エンコードデータの取得に失敗した");
            produced += 1;
        }

        assert_eq!(
            produced,
            switch_at + trailing,
            "出力数が入力数と一致しない: {produced} != {}",
            switch_at + trailing
        );
        Ok(())
    })?;

    assert!(
        changed.get() > 0,
        "ビットレート変更を伴うケースが 1 件も無い\n{runner}"
    );
    Ok(())
}

/// 同じビットレートで連続 reconfigure してもエンコードが完走すること (冪等性の緩い形)
#[test]
fn reconfigure_same_bitrate_is_safe() -> noprop::TestResult {
    let seed = noprop::seed_from_env_or_time("SHIGUREDO_AOM_PBT_SEED")?;
    // 複数回繰り返したケース数を数える
    //
    // repeats = 1 ばかりだと「連続」呼び出しを検証したことにならないため、
    // 空振り防止のゲートとして使う。
    let repeated = Cell::new(0usize);
    let mut runner = noprop::Runner::new(seed);

    runner.run(CASES, |ctx| {
        let bitrate = sample_bitrate(ctx);
        let repeats = noprop::sample_usize_in(ctx, REPEATS_RANGE);

        let width = 160u32;
        let height = 120u32;
        let mut encoder = Encoder::new(realtime_cbr(width, height, bitrate))
            .expect("エンコーダーの生成に失敗した");

        for _ in 0..repeats {
            encoder
                .reconfigure(ReconfigureParams {
                    rc_target_bitrate: Some(bitrate),
                })
                .expect("同値での再設定に失敗した");
        }

        if repeats > 1 {
            repeated.set(repeated.get() + 1);
        }

        let produced = encode_count(&mut encoder, width, height, 3);
        assert_eq!(produced, 3, "出力数が 3 にならない: {produced}");
        Ok(())
    })?;

    assert!(
        repeated.get() > 0,
        "複数回繰り返したケースが 1 件も無い\n{runner}"
    );
    Ok(())
}
