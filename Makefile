.PHONY: start download build run

start: download build run

download:
	cargo fetch

build:
	cargo build --release

run:
	cargo run --release
