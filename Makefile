.PHONY: run dev serve build clean deploy

run:
	cargo run

dev:
	cargo run --features dev

serve:
	trunk serve --features web

build:
	trunk build --release --public-url /raycer/ --features web && cp _headers dist/

clean:
	cargo clean
	rm -rf dist/
