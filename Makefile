.PHONY: run run-dev serve build clean

run:
	cargo run

run-dev:
	cargo run --features dev

serve:
	trunk serve --features web

build:
	trunk build --release --public-url /raycer/ --features web

clean:
	cargo clean
