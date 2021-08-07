
.PHONY: build
build:
	wasm-pack build --no-typescript --target web

.PHONY: run
run: build
	python -m SimpleHTTPServer &
	open http://localhost:8000/
