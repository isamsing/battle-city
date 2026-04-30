.PHONY: start download build run wasm deploy

start: download build run

download:
	cargo fetch

build:
	cargo build --release

run:
	cargo run --release

wasm:
	trunk build --release

deploy:
	fly deploy
