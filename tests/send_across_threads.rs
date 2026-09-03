// 統合テスト (tests/*.rs) はファイルごとに独立したバイナリとしてコンパイルされる。
// 本テストは共有ヘルパー (tests/helpers/) の一部の関数しか使わないため、
// 本バイナリからは未使用のヘルパー関数が存在する。この expect はその警告を
// 意図的に許容する。すべてのヘルパー関数を使うようになった場合は expect が
// 未達になり、削除が必要になる。
#![expect(dead_code)]

use shiguredo_aom::{Decoder, DecoderConfig, EncodeOptions, Encoder, RateControlMode};

#[path = "helpers/helpers.rs"]
mod helpers;
use helpers::*;

// 子スレッドからの受信待ちの上限
//
// 320x240 12 フレームのエンコード・デコードは通常 1 秒未満で完了するため、
// CI 環境の性能変動を考慮しても 30 秒は十分なマージンがある。
const RECV_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// 子スレッドからの受信をタイムアウト付きで行い、完了を join で確認する
///
/// 受信を先に行うことで、子スレッドのハングをタイムアウトによる失敗として検出する。
/// ハングした子スレッドの join はブロックするため、タイムアウト時は join を呼ばない。
/// 切断時は子スレッドが panic した可能性があるため join で検査する。
fn recv_with_timeout<T>(
    rx: std::sync::mpsc::Receiver<T>,
    handle: std::thread::JoinHandle<()>,
    target: &str,
) -> T {
    match rx.recv_timeout(RECV_TIMEOUT) {
        Ok(value) => {
            // 受信成功後に子スレッドの panic を検査する
            handle.join().expect("スレッド A がパニックした");
            value
        }
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            panic!("{target}の受信がタイムアウトした (子スレッドがハングしている可能性)");
        }
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            // 送信前に子スレッドが終了した場合は join で panic の有無を検査する
            handle.join().expect("スレッド A がパニックした");
            unreachable!("切断後に join が成功することはないはず");
        }
    }
}

// ============================================================================
// スレッド間移動テスト
// ============================================================================

/// エンコーダーをスレッド間移動させても、エンコードが正常に動作することを検証する
///
/// libaom の内部ワーカースレッド (CONFIG_MULTITHREAD=1) が生成・動作した状態で
/// エンコーダーを別スレッドに移動し、移動後のエンコード結果がデコード可能で
/// 入力フレーム数と一致することを確認する。
#[test]
fn test_encoder_send_across_threads() {
    let width = 320;
    let height = 240;
    let num_frames = 12;
    // スレッド A 側でワーカーを動作させて移動前の出力を確認できる数、かつ
    // メイン側でも十分なフレーム数を検証できる数に分割する
    let thread_a_frames = 4;

    // ワーカーが実際に並列処理するよう row_mt とタイルを有効にする。
    // タイル列数は libaom の制約により設定値と一致しない場合がある (内部実装依存で、バージョン更新で変わり得る)
    let mut config = realtime_config(width, height, RateControlMode::Cbr);
    config.g_threads = Some(4);
    config.row_mt = Some(true);
    config.tile_columns = Some(2);
    config.tile_rows = Some(1);
    let options = EncodeOptions {
        force_keyframe: false,
    };

    let (tx, rx) = std::sync::mpsc::channel();
    let options_in_thread_a = options.clone();

    let handle = std::thread::spawn(move || {
        let mut encoder = Encoder::new(config).expect("エンコーダーの生成に失敗した");
        let mut packets_in_thread_a = Vec::new();

        // ワーカースレッドを生成・動作させるため、スレッド A でエンコードを開始する。
        // ワーカースレッドの生成は外部から観測できないため、スレッド A でのパケット出力を代理指標とする
        drive_dummy(
            &mut encoder,
            &options_in_thread_a,
            width,
            height,
            0..thread_a_frames,
            &mut packets_in_thread_a,
        );

        // 移動前にスレッド A でエンコード結果が出力されていることを確認する
        assert!(
            !packets_in_thread_a.is_empty(),
            "移動前にエンコード結果が出力されていない"
        );

        // エンコーダーと移動前のパケットをメインスレッドに送信する
        tx.send((encoder, packets_in_thread_a))
            .expect("エンコーダーの送信に失敗した");
    });

    // 受信を先に行い、子スレッドの完了は受信後に join で確認する。
    // 受信にタイムアウトを付けることで、子スレッドのハングを失敗として検出する。
    //
    // メインスレッドで残りのフレームをエンコードする。
    // スレッド A 側のパケットを起点に、メインスレッドのエンコード結果と finish 後のドレインを追記して全パケットを結合する
    let (mut encoder, mut all_packets) = recv_with_timeout(rx, handle, "エンコーダー");
    let mut packets_after_move = Vec::new();
    drive_dummy(
        &mut encoder,
        &options,
        width,
        height,
        thread_a_frames..num_frames,
        &mut packets_after_move,
    );
    // 移動後のエンコードでも 1 つ以上のパケットが出力されていることを確認する
    assert!(
        !packets_after_move.is_empty(),
        "移動後にエンコード結果が出力されていない"
    );
    all_packets.extend(packets_after_move);
    drain_after_finish(&mut encoder, &mut all_packets);

    // 移動後のエンコード結果がデコード可能で、入力フレーム数と一致することを確認する
    let decoded = decode_frames(&all_packets);
    assert_eq!(
        decoded.len(),
        num_frames,
        "デコードフレーム数が入力フレーム数 ({num_frames}) と一致しない"
    );
    for (i, (y, w, h)) in decoded.iter().enumerate() {
        assert_eq!(*w, width as usize, "フレーム {i}: 幅が一致しない");
        assert_eq!(*h, height as usize, "フレーム {i}: 高さが一致しない");
        assert!(!y.is_empty(), "フレーム {i}: Y プレーンが空");
    }
}

/// デコーダーをスレッド間移動させても、デコードが正常に動作することを検証する
///
/// libaom の内部ワーカースレッド (CONFIG_MULTITHREAD=1) が生成・動作した状態で
/// デコーダーを別スレッドに移動し、移動後もデコードが継続できることを確認する。
#[test]
fn test_decoder_send_across_threads() {
    let width = 320;
    let height = 240;
    let num_frames = 12;

    // メインスレッドで入力フレームを事前にエンコードしてパケット列を用意する。
    // デコーダーのワーカーがタイル並列で動作するよう、タイル付きでエンコードする。
    let mut config = realtime_config(width, height, RateControlMode::Cbr);
    config.row_mt = Some(true);
    config.tile_columns = Some(2);
    config.tile_rows = Some(1);
    let input_frames: Vec<(Vec<u8>, Vec<u8>, Vec<u8>)> = (0..num_frames)
        .map(|i| generate_dummy_i420(width as usize, height as usize, i))
        .collect();
    let packets = encode_frames(config, &input_frames);
    assert!(!packets.is_empty(), "エンコード結果のパケットが空");

    // パケット列を半分に分割する。packets.len() < 2 だと分割ポイントが 0 になり
    // スレッド A が 0 パケットをデコードしてしまうため、防御的に assert する
    let split_point = packets.len() / 2;
    assert!(
        split_point >= 1,
        "パケット列が短く分割ポイントが 0 になっている"
    );
    let thread_a_packets = packets[..split_point].to_vec();
    let main_packets = &packets[split_point..];
    let (tx, rx) = std::sync::mpsc::channel();

    let handle = std::thread::spawn(move || {
        let mut decoder_config = DecoderConfig::new();
        decoder_config.threads = Some(4);
        let mut decoder = Decoder::new(decoder_config).expect("デコーダーの生成に失敗した");

        // ワーカースレッドを生成・動作させるため、スレッド A でデコードを開始する。
        // ワーカースレッドの生成は外部から観測できないため、スレッド A での出力を代理指標とする
        let mut decoded_in_thread_a = 0usize;
        for packet in &thread_a_packets {
            decoder.decode(packet).expect("デコードに失敗した");
            while let Some(frame) = decoder.next_frame() {
                assert_eq!(
                    frame.width(),
                    width as usize,
                    "移動前のデコード結果の幅が一致しない"
                );
                assert_eq!(
                    frame.height(),
                    height as usize,
                    "移動前のデコード結果の高さが一致しない"
                );
                assert!(
                    frame.y_plane().is_ok_and(|y| !y.is_empty()),
                    "移動前のデコード結果の Y プレーンが空"
                );
                decoded_in_thread_a += 1;
            }
        }

        // 移動前にスレッド A でデコード結果が出力されていることを確認する
        assert!(
            decoded_in_thread_a > 0,
            "移動前にデコード結果が出力されていない"
        );

        // デコーダーとスレッド A 側のデコードフレーム数をメインスレッドに送信する
        tx.send((decoder, decoded_in_thread_a))
            .expect("デコーダーの送信に失敗した");
    });

    // メインスレッドで残りのパケットをデコードする。
    // 受信を先に行い、子スレッドの完了は受信後に join で確認する。
    // 受信にタイムアウトを付けることで、子スレッドのハングを失敗として検出する。
    let (mut decoder, decoded_in_thread_a) = recv_with_timeout(rx, handle, "デコーダー");
    let mut decoded_after_move: Vec<(Vec<u8>, usize, usize)> = Vec::new();
    for packet in main_packets {
        decoder.decode(packet).expect("移動後のデコードに失敗した");
        while let Some(frame) = decoder.next_frame() {
            decoded_after_move.push((extract_y_plane(&frame), frame.width(), frame.height()));
        }
    }

    // finish と finish 後のドレインが正常動作することを確認する
    decoder.finish().expect("finish に失敗した");
    while let Some(frame) = decoder.next_frame() {
        decoded_after_move.push((extract_y_plane(&frame), frame.width(), frame.height()));
    }

    // スレッド A 側の分と合わせて、デコードフレーム数が入力フレーム数と一致することを確認する
    assert_eq!(
        decoded_in_thread_a + decoded_after_move.len(),
        num_frames,
        "デコードフレーム数が入力フレーム数 ({num_frames}) と一致しない"
    );

    // 各フレームの幅・高さが入力フレームの解像度と一致し、Y プレーンが非空であることを確認する
    for (i, (y, w, h)) in decoded_after_move.iter().enumerate() {
        assert_eq!(*w, width as usize, "フレーム {i}: 幅が一致しない");
        assert_eq!(*h, height as usize, "フレーム {i}: 高さが一致しない");
        assert!(!y.is_empty(), "フレーム {i}: Y プレーンが空");
    }
}
