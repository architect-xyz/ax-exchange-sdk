lint: 
	cargo clippy --benches --examples --tests -- -D warnings 
fmt:
	cargo clippy --workspace --all-targets --tests --fix --allow-dirty -- -D warnings
	cargo fmt --all
build:
	cargo build
test:
	cargo test -- --nocapture
all: fmt lint build test
