.PHONY: test cover check clippy fmt fmt-check clean

# 全テストを実行する
test:
	cargo test --workspace --features source-build

# 全テストカバレッジ付きで実行する
cover:
	cargo llvm-cov --tests --workspace --features source-build

# cargo check を実行する
check:
	cargo check --workspace

# cargo clippy を実行する
clippy:
	cargo clippy --workspace --all-targets --features source-build -- -D warnings

# cargo fmt を実行する（フォーマットを適用）
fmt:
	cargo fmt --all

# cargo fmt を検査する（フォーマットを変更しない）
fmt-check:
	cargo fmt --all -- --check

# ビルド成果物を削除する
clean:
	cargo clean
