.PHONY: build test lint clean docs release \
       build-server build-collector build-c-bindings build-wasm \
       build-android build-ios \
       test-core test-storage test-all \
       run-server run-collector \
       docker-build

CARGO := cargo
TARGET_DIR := target
RELEASE_DIR := release

build: build-server build-collector

build-server:
	$(CARGO) build --bin duanes-server --release

build-collector:
	$(CARGO) build --bin duanes-collector --release

build-telemetry-processor:
	$(CARGO) build --bin duanes-telemetry-processor --release

build-c-bindings:
	$(CARGO) build --release -p duanes-c

build-wasm:
	wasm-pack build bindings/duanes-wasm --target web --out-dir pkg

build-android:
	$(CARGO) ndk -t arm64-v8a -t x86_64 -o build/android build --release -p duanes-android

build-ios:
	$(CARGO) lipo --release -p duanes-ios --output build/ios/libduanes_ios.a

test: test-all

test-all:
	$(CARGO) test --workspace

test-core:
	$(CARGO) test -p duanes-core

test-storage:
	$(CARGO) test -p duanes-storage

lint:
	$(CARGO) clippy --workspace --all-targets -- -D warnings
	$(CARGO) fmt --check

fmt:
	$(CARGO) fmt

clean:
	$(CARGO) clean
	rm -rf $(RELEASE_DIR) build/

docs:
	$(CARGO) doc --workspace --no-deps

run-server: build-server
	./$(TARGET_DIR)/release/duanes-server --config config/duanes.example.toml

run-collector: build-collector
	./$(TARGET_DIR)/release/duanes-collector

release: lint test-all
	$(CARGO) build --release --workspace
	mkdir -p $(RELEASE_DIR)
	cp $(TARGET_DIR)/release/duanes-server $(RELEASE_DIR)/
	cp $(TARGET_DIR)/release/duanes-collector $(RELEASE_DIR)/
	cp $(TARGET_DIR)/release/duanes-telemetry-processor $(RELEASE_DIR)/

docker-build:
	docker build -t duanes-server -f docker/Dockerfile.server .
	docker build -t duanes-collector -f docker/Dockerfile.collector .

SDK_PYTHON_DIR := sdks/python
SDK_TS_DIR := sdks/typescript

sdk-python:
	cd $(SDK_PYTHON_DIR) && python -m build

sdk-typescript:
	cd $(SDK_TS_DIR) && npm run build

dashboard-dev:
	cd services/dashboard && npm run dev

dashboard-build:
	cd services/dashboard && npm run build
