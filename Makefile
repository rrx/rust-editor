default:
	cargo build
	cargo fmt

test:
	cargo test --all -- --nocapture
