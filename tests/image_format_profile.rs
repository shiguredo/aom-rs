// 統合テスト (tests/*.rs) はファイルごとに独立したバイナリとしてコンパイルされる。
// 本テストは共有ヘルパー (tests/helpers/) の一部の関数しか使わないため、
// 本バイナリからは未使用のヘルパー関数が存在する。この expect はその警告を
// 意図的に許容する。すべてのヘルパー関数を使うようになった場合は expect が
// 未達になり、削除が必要になる。
#![expect(dead_code)]

use shiguredo_aom::{
    EncodeOptions, Encoder, EncoderConfig, ImageData, ImageFormat, RateControlMode,
};

#[path = "helpers/helpers.rs"]
mod helpers;
use helpers::*;

// ============================================================================
// 画像フォーマット × g_profile 検証テスト
// ============================================================================

/// 設定変更用の関数型エイリアス
type Setter = fn(&mut EncoderConfig);

/// 不正な image_format × g_profile の組み合わせ (代表例) で
/// `Encoder::new` が事前検証のエラーを返すことを確認する
///
/// 全セルテスト (test_validate_all_cells_8_to_10bit) が is_err のみを確認する
/// のに対し、本テストはエラーメッセージに事前検証の reason が含まれること
/// (libaom の init エラーではなく aom-rs の事前検証エラーであること) を
/// 確認する。
#[test]
fn test_encoder_new_rejects_invalid_format_profile() {
    let cases: &[(&str, Setter)] = &[
        ("I422 + profile 0 (デフォルト)", |c| {
            c.image_format = ImageFormat::I422;
        }),
        ("I444 + profile 0 (デフォルト)", |c| {
            c.image_format = ImageFormat::I444;
        }),
        ("I444 + profile 2 (8-bit)", |c| {
            c.image_format = ImageFormat::I444;
            c.g_profile = 2;
        }),
        ("I420 + profile 1", |c| c.g_profile = 1),
        ("I420 + profile 2 (8-bit)", |c| c.g_profile = 2),
        ("Yv12 + profile 1", |c| {
            c.image_format = ImageFormat::Yv12;
            c.g_profile = 1;
        }),
        ("I444 + profile 2 + monochrome", |c| {
            c.image_format = ImageFormat::I444;
            c.g_profile = 2;
            c.monochrome = Some(true);
        }),
    ];

    for (name, set) in cases {
        let mut config = good_quality_config(320, 240, RateControlMode::Cbr);
        set(&mut config);
        let err = Encoder::new(config).expect_err(name);
        assert!(
            err.to_string()
                .contains("invalid image format / g_profile combination"),
            "{name}: 予期しないエラーメッセージ: {err}"
        );
    }
}

/// 正しい組み合わせ (代表例) でエンコードまで成功することを確認する
///
/// 完了条件「正しい組み合わせ (代表例) でエンコードが成功すること」の検証。
/// I420 + profile 0 は既存の roundtrip テストがカバーしているため、
/// ここでは既存テストに存在しない I422 / I444 (monochrome 含む) の
/// エンコード経路を固定する。
#[test]
fn test_valid_combinations_encode_success() {
    let mut config = realtime_config(320, 240, RateControlMode::Cbr);
    config.image_format = ImageFormat::I422;
    config.g_profile = 2;
    encode_one_frame(config);

    let mut config = realtime_config(320, 240, RateControlMode::Cbr);
    config.image_format = ImageFormat::I444;
    config.g_profile = 1;
    encode_one_frame(config);

    let mut config = realtime_config(320, 240, RateControlMode::Cbr);
    config.image_format = ImageFormat::I444;
    config.monochrome = Some(true);
    encode_one_frame(config);
}

/// ダミーフレームを 1 フレームだけエンコードして成功することを確認する
///
/// 画質は検証しない (128 固定の平坦なフレームをエンコードするだけ)。
/// フォーマットは `config.image_format` (I422 / I444) に従う。
fn encode_one_frame(config: EncoderConfig) {
    let format = config.image_format;
    let width = config.g_w as usize;
    let height = config.g_h as usize;
    let (uv_width, uv_height) = match format {
        ImageFormat::I422 => (width.div_ceil(2), height),
        ImageFormat::I444 => (width, height),
        _ => panic!("encode_one_frame: 対応しないフォーマット: {format:?}"),
    };
    let y = vec![128u8; width * height];
    let u = vec![128u8; uv_width * uv_height];
    let v = vec![128u8; uv_width * uv_height];

    let mut encoder = Encoder::new(config).expect("エンコーダーの生成に失敗しました");
    let image = match format {
        ImageFormat::I422 => ImageData::I422 {
            y: &y,
            u: &u,
            v: &v,
        },
        ImageFormat::I444 => ImageData::I444 {
            y: &y,
            u: &u,
            v: &v,
        },
        _ => unreachable!(),
    };
    let options = EncodeOptions {
        force_keyframe: false,
    };
    encoder
        .encode(&image, &options)
        .expect("エンコードに失敗しました");
    while let Some(encoded) = encoder.next_frame() {
        let _ = encoded
            .data()
            .expect("エンコードデータの取得に失敗しました");
    }
    drain_after_finish(&mut encoder, &mut Vec::new());
}

/// 有効ビット深度 8-10 の判定表
///
/// (フォーマット, monochrome, 有効な profile) の組み合わせを AV1 仕様の
/// profile 定義と libaom の encode 時検証条件から独立にハードコードした表。
/// 検証ロジック (validate_image_format_profile) の複製ではなく、期待値の
/// 独立した情報源として使う。
const VALID_PROFILE_TABLE: &[(ImageFormat, bool, u32)] = &[
    (ImageFormat::I420, false, 0),
    (ImageFormat::I420, true, 0),
    (ImageFormat::Yv12, false, 0),
    (ImageFormat::Yv12, true, 0),
    (ImageFormat::Nv12, false, 0),
    (ImageFormat::Nv12, true, 0),
    (ImageFormat::I422, false, 2),
    (ImageFormat::I422, true, 2),
    (ImageFormat::I444, false, 1),
    (ImageFormat::I444, true, 0),
    (ImageFormat::I42016, false, 0),
    (ImageFormat::I42016, true, 0),
    (ImageFormat::I42216, false, 2),
    (ImageFormat::I42216, true, 2),
    (ImageFormat::I44416, false, 1),
    (ImageFormat::I44416, true, 0),
];

/// 有効ビット深度 8-10 の判定表の全セルを検証する
///
/// 各フォーマット × 有効ビット深度 × monochrome の組み合わせについて、
/// 判定表 (VALID_PROFILE_TABLE) が示す profile でのみ `Encoder::new` が成功し、
/// 他の profile ではエラーになることを確認する。
///
/// 8-bit フォーマットの有効ビット深度は常に 8 (g_bit_depth の指定は判定に
/// 使わない)。16-bit フォーマットは g_bit_depth (8 または 10) が有効ビット
/// 深度になり、判定は 8-bit フォーマットと同じになる。
///
/// 注意: 拒否セルの中には、事前検証を通過して libaom の init 時検証
/// (validate_config の monochrome 制約) が拒否するもの
/// (I444 / I44416 + monochrome + profile 1) が含まれる。エラーの出自は
/// 代表ケーステスト (test_encoder_new_rejects_invalid_format_profile) と
/// test_deferred_cases_rejected_by_libaom_init が検証するため、本テストでは
/// エラーの出自を問わず is_err のみを確認する。
#[test]
fn test_validate_all_cells_8_to_10bit() {
    for &(format, monochrome, valid_profile) in VALID_PROFILE_TABLE {
        let is_16bit = matches!(
            format,
            ImageFormat::I42016 | ImageFormat::I42216 | ImageFormat::I44416
        );
        // 8-bit フォーマットは有効ビット深度が 8 固定のため、ループは 1 回でよい
        let bit_depths: &[u32] = if is_16bit { &[8, 10] } else { &[8] };
        for &bit_depth in bit_depths {
            for profile in 0..=2 {
                let mut config = realtime_config(64, 64, RateControlMode::Cbr);
                config.image_format = format;
                config.g_bit_depth = if is_16bit { Some(bit_depth) } else { None };
                config.g_profile = profile;
                config.monochrome = Some(monochrome);

                let result = Encoder::new(config);
                if profile == valid_profile {
                    assert!(
                        result.is_ok(),
                        "{format:?} + {bit_depth}-bit + monochrome={monochrome} + profile {profile} は成功するはず"
                    );
                } else {
                    assert!(
                        result.is_err(),
                        "{format:?} + {bit_depth}-bit + monochrome={monochrome} + profile {profile} は拒否されるはず"
                    );
                }
            }
        }
    }
}

/// 検証対象外の組み合わせが libaom の init でエラーになることを確認する
///
/// 事前検証の対象外とした 3 ケースは、aom-rs の事前検証を通過した後も
/// libaom が init 時に拒否する。「init 成功 → 遅延失敗」にならないこと
/// (init でエラーになること) を固定する。エラーに aom-rs の事前検証の
/// reason が含まれないことも確認し、出自が libaom の init 検証である
/// (事前検証が誤って拒否していない) ことを担保する。
#[test]
fn test_deferred_cases_rejected_by_libaom_init() {
    // 8-bit フォーマット + g_bit_depth > 8
    let mut config = good_quality_config(320, 240, RateControlMode::Cbr);
    config.g_bit_depth = Some(10);
    assert_deferred_rejection(
        Encoder::new(config),
        "8-bit フォーマット + g_bit_depth > 8 は libaom の init で拒否されるはず",
    );

    // 12-bit + profile < 2
    let config = highbitdepth_config(320, 240, ImageFormat::I42016, 12, 0);
    assert_deferred_rejection(
        Encoder::new(config),
        "12-bit + profile < 2 は libaom の init で拒否されるはず",
    );

    // profile 1 + monochrome
    let mut config = good_quality_config(320, 240, RateControlMode::Cbr);
    config.image_format = ImageFormat::I444;
    config.g_profile = 1;
    config.monochrome = Some(true);
    assert_deferred_rejection(
        Encoder::new(config),
        "profile 1 + monochrome は libaom の init で拒否されるはず",
    );
}

/// 検証対象外ケースのエラーが libaom の init 検証によるものであることを確認する
///
/// エラーには aom-rs の事前検証の reason が含まれないこと
/// (事前検証は通過していること) を確認する。
fn assert_deferred_rejection(result: Result<Encoder, shiguredo_aom::Error>, message: &str) {
    let err = result.expect_err(message);
    assert!(
        !err.to_string()
            .contains("invalid image format / g_profile combination"),
        "{message}: 事前検証が誤って拒否しています: {err}"
    );
}

/// 12-bit (I42016 + g_bit_depth: Some(12) + g_profile: 2) が
/// 事前検証で誤って拒否されないことを確認する
///
/// 12-bit は profile < 2 を libaom が init 時に拒否するため、
/// 事前検証の対象外としている。
#[test]
fn test_encoder_new_accepts_12bit() {
    let config = highbitdepth_config(320, 240, ImageFormat::I42016, 12, 2);
    Encoder::new(config).expect("I42016 + 12-bit + profile 2 は成功するはず");
}

/// 16-bit フォーマットで g_bit_depth 未指定 (None) のケースが
/// 有効ビット深度 8 として検証されることを確認する
///
/// `g_bit_depth: None` は有効ビット深度 8 と同じ判定になるため、
/// 4:2:0 系は profile 0 のみ許可される。
#[test]
fn test_encoder_new_accepts_16bit_without_bit_depth() {
    // g_bit_depth 未指定 (= 有効ビット深度 8) + I42016 + profile 0 は有効
    let mut config = realtime_config(320, 240, RateControlMode::Cbr);
    config.image_format = ImageFormat::I42016;
    Encoder::new(config).expect("I42016 + g_bit_depth 未指定 + profile 0 は成功するはず");

    // g_bit_depth 未指定 + I42016 + profile 1 は不正 (4:2:0 系は profile 0 のみ)
    let mut config = realtime_config(320, 240, RateControlMode::Cbr);
    config.image_format = ImageFormat::I42016;
    config.g_profile = 1;
    let err =
        Encoder::new(config).expect_err("I42016 + g_bit_depth 未指定 + profile 1 は拒否されるはず");
    assert!(
        err.to_string()
            .contains("invalid image format / g_profile combination"),
        "予期しないエラーメッセージ: {err}"
    );
}
