# iOS / Android 向けの prebuilt を追加する

- Created: 2026-10-03
- Completed: {YYYY-MM-DD}
- Branch: feature/add-mobile-prebuilt
- Polished: {YYYY-MM-DD}

## 目的

モバイル向けの Rust アプリケーションでも、libaom をソースからビルドせずに prebuilt で利用できるようにする。
ユーザーからの「opus-rs と同様に Android / iOS 向けの prebuilt を用意したい」という要望に対応する。

## 現状

- `build.rs` の `get_target_platform` は Linux、macOS、Windows 向けのみを扱い、iOS / Android のターゲットを指定すると panic する
- `build.rs` の `rewrite_symbols` は Mach-O のシンボル先頭の `_` を `CARGO_CFG_TARGET_OS == "macos"` の場合だけ処理しており、iOS を追加するとシンボル書き換えが壊れる
- `build.rs` の `build_from_source` に iOS の Xcode SDK と Android NDK を使う設定がなく、モバイル向けのソースビルドができない
- `.github/workflows/ci.yml` と `.github/workflows/release.yml` にモバイル向けのビルドジョブがない
- `README.md` に iOS / Android 向けのビルド手順と prebuilt の記述がない

## 設計方針

### 対象ターゲット

| 対象 | Rust ターゲット | prebuilt のプラットフォーム名 |
| --- | --- | --- |
| iOS 実機 arm64 | `aarch64-apple-ios` | `ios_arm64` |
| iOS シミュレーター arm64 | `aarch64-apple-ios-sim` | `ios-sim_arm64` |
| iOS シミュレーター x86_64 | `x86_64-apple-ios` | `ios-sim_x86_64` |
| Android arm64-v8a | `aarch64-linux-android` | `android_arm64` |
| Android x86_64 | `x86_64-linux-android` | `android_x86_64` |

### ビルド

- iOS の prebuilt は実機と x86_64 シミュレーターが iOS 13.0 以降、arm64 シミュレーターが iOS 14.0 以降を対象とする
- Android の prebuilt は arm64-v8a と x86_64 の 2 ABI、API level 21 以降を対象とし、NDK `28.2.13676358` でビルドする
- iOS では Xcode の SDK パスを `xcrun` で解決し、CMake と bindgen に同じ SDK、アーキテクチャ、最小 OS バージョンを指定する
  - libaom の CMake は `AOM_TARGET_SYSTEM` が `Darwin` のときだけ ARM アセンブリの処理と x86_64 の NASM オブジェクト形式 (`macho64`) を切り替えるため、`CMAKE_SYSTEM_NAME` に `Darwin` を指定する
  - Darwin ホストでは `CMAKE_SYSTEM_NAME` が同じだと `CMAKE_SYSTEM_PROCESSOR` が空になり libaom が CPU を generic と判定して SIMD が無効になるため、`AOM_TARGET_CPU` を明示する
  - ターゲットの区別は `CMAKE_OSX_SYSROOT` (iphoneos / iphonesimulator) と `CMAKE_OSX_ARCHITECTURES` で行う
- Android では NDK 付属の `android.toolchain.cmake` を使い、`ANDROID_ABI`、`ANDROID_PLATFORM`、`ANDROID_STL=c++_static` を指定する
  - libaom の補助ライブラリ `aom_av1_rc` は C++ ソースを含むため C++ 標準ライブラリが必要になる
  - 共有ライブラリへの静的リンクに備えて `CONFIG_PIC=1` で PIC を有効にする
- Mach-O のシンボル書き換えは Apple プラットフォーム判定 (`CARGO_CFG_TARGET_VENDOR == "apple"`) に変更して iOS でも動くようにする
- `build-dependencies` に `cc` を追加し、cmake クレートのコンパイラ判定に Android NDK の C / C++ コンパイラを指定する

### 配布と検証

- `.github/workflows/mobile.yml` を追加し、CI とリリースから共用する
- ワークフローではソースビルド、Rust のリンク、シンボルのプレフィックス検証、アーカイブ生成、SHA256 検証、展開物の一致確認を行う
- アーカイブにはシンボル書き換え済みの `lib/libaom.a`、`bindings.rs`、libaom の `LICENSE` と `PATENTS` を収録する
- リリースではアップロード後に prebuilt の自動選択とリンクを検証し、成功を `publish` の前提にする

## 完了条件

- 全 5 ターゲットで静的ライブラリとバインディングを生成できる
- 全定義済み外部シンボルが、Mach-O 固有の先頭 `_` を除いて `shiguredo_aom_` プレフィックスを持つ
- 各アーカイブの SHA256 が一致し、収録したライブラリとバインディングを用いて Rust のリンクが成功する
- GitHub Actions の CI でモバイル向けビルドとリンクの検証が通る
- 次回リリースでアーカイブとチェックサムのアップロード、公開された prebuilt の自動選択とリンクの検証、crates.io への公開の順に進むワークフローを構成し、検証に失敗した場合は `publish` を開始しない
- 既存の単体テスト、PBT、フォーマット、Clippy が通る
