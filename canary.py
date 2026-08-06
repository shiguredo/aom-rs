import argparse
import re
import subprocess
from typing import Optional

# [package] セクション内の version 行 (行頭のフィールド名完全一致) を検出する。
# 行頭アンカーにより rust-version 等の version を含む他のフィールドへの誤マッチを
# 防ぐ (\bversion\b は rust-version 内の version にもマッチするため不可)。
VERSION_LINE_RE = re.compile(r'^\s*version\s*=\s*"([\w.-]+)"', re.MULTILINE)


# ファイルを読み込み、バージョンを更新
def update_version(file_path: str, dry_run: bool) -> Optional[str]:
    with open(file_path, "r", encoding="utf-8") as f:
        content: str = f.read()

    # [package] セクションの開始位置を見つける。
    # 行頭アンカーで検出し、コメント内の [package] 文字列への誤マッチを防ぐ
    # (release.yml のバージョン照合と同じ定義)。
    package_match = re.search(r"^\s*\[package\]", content, re.MULTILINE)
    if not package_match:
        raise ValueError("[package] section not found in Cargo.toml")
    package_start = package_match.start()
    # 次のセクション ([package.metadata...] / [dependencies] など) の開始位置を見つける。
    # 任意の [ セクションで境界にする (release.yml のバージョン照合と同じ定義)。
    # [package] 行自体にマッチしないよう、[package] 行の終端から検索する。
    next_section = re.search(r"^\[", content[package_match.end() :], re.MULTILINE)
    if next_section:
        package_end = package_match.end() + next_section.start()
        package_content = content[package_start:package_end]
    else:
        package_content = content[package_start:]

    current_version_match = VERSION_LINE_RE.search(package_content)
    if not current_version_match:
        raise ValueError("Version not found in [package] section of Cargo.toml")

    current_version: str = current_version_match.group(1)

    # [package] セクション内の version 行のみを更新
    if "-canary." in current_version:
        updated_package, count = re.subn(
            r'^(\s*version\s*=\s*")(\d+\.\d+\.\d+-canary\.)(\d+)',
            lambda m: f"{m.group(1)}{m.group(2)}{int(m.group(3)) + 1}",
            package_content,
            count=1,  # 最初の 1 つだけを更新
            flags=re.MULTILINE,
        )
    else:
        # -canary.X がない場合、次のマイナーバージョンにして -canary.0 を追加
        updated_package, count = re.subn(
            r'^(\s*version\s*=\s*")(\d+)\.(\d+)\.(\d+)',
            lambda m: f"{m.group(1)}{m.group(2)}.{int(m.group(3)) + 1}.0-canary.0",
            package_content,
            count=1,  # 最初の 1 つだけを更新
            flags=re.MULTILINE,
        )

    if count == 0:
        raise ValueError(f"Version format not supported: {current_version}")

    # 元のコンテンツの [package] セクション部分を更新後の内容に置き換える
    if next_section:
        new_content = content[:package_start] + updated_package + content[package_end:]
    else:
        new_content = content[:package_start] + updated_package

    # 新しいバージョンを抽出する。
    # 変換 (subn) は count > 0 の時点で必ず version 行を更新済みのため、
    # 抽出に失敗する経路は存在しない (失敗系は count == 0 チェックが担う)。
    updated_version_match = VERSION_LINE_RE.search(updated_package)
    if updated_version_match is None:
        raise ValueError("Version line was not found in [package] section")

    new_version: str = updated_version_match.group(1)

    print(f"Current version: {current_version}")
    print(f"New version: {new_version}")
    confirmation: str = (
        input("Do you want to update the version? (Y/n): ").strip().lower()
    )

    if confirmation not in ("y", ""):
        print("Version update canceled.")
        return None

    # Dry-run 時の動作
    if dry_run:
        print("Dry-run: Version would be updated to:")
        print(new_content)
    else:
        with open(file_path, "w", encoding="utf-8") as f:
            f.write(new_content)
        print(f"Version updated in Cargo.toml to {new_version}")

    return new_version


# cargo update shiguredo_aom を実行
def run_cargo_update(dry_run: bool) -> None:
    if dry_run:
        print("Dry-run: Would run 'cargo update shiguredo_aom'")
    else:
        subprocess.run(["cargo", "update", "shiguredo_aom"], check=True)
        print("cargo update shiguredo_aom executed")


# git コミットを実行
def git_commit_version(new_version: str, dry_run: bool) -> None:
    if dry_run:
        print("Dry-run: Would run 'git add Cargo.toml Cargo.lock'")
        print(f"Dry-run: Would run '[canary] Bump version to {new_version}'")
    else:
        subprocess.run(["git", "add", "Cargo.toml", "Cargo.lock"], check=True)
        subprocess.run(
            ["git", "commit", "-m", f"[canary] Bump version to {new_version}"],
            check=True,
        )
        print(f"Version bumped and committed: {new_version}")


# git タグ付け、プッシュを実行
def git_operations_after_build(new_version: str, dry_run: bool) -> None:
    if dry_run:
        print(f"Dry-run: Would run 'git tag {new_version}'")
        print("Dry-run: Would run 'git push'")
        print(f"Dry-run: Would run 'git push origin {new_version}'")
    else:
        subprocess.run(["git", "tag", new_version], check=True)
        subprocess.run(["git", "push"], check=True)
        subprocess.run(["git", "push", "origin", new_version], check=True)


# メイン処理
def main() -> None:
    parser = argparse.ArgumentParser(
        description="Update Cargo.toml version and commit changes."
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Run in dry-run mode without making actual changes",
    )
    args = parser.parse_args()

    cargo_toml_path: str = "Cargo.toml"

    # バージョン更新
    new_version: Optional[str] = update_version(cargo_toml_path, args.dry_run)

    if not new_version:
        return  # ユーザーが確認をキャンセルした場合、処理を中断

    # cargo update shiguredo_aom を実行
    run_cargo_update(args.dry_run)

    # バージョン更新後に git commit
    git_commit_version(new_version, args.dry_run)

    # git タグ付け、プッシュ
    git_operations_after_build(new_version, args.dry_run)


if __name__ == "__main__":
    main()
