# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Streaming support (SSE passthrough with token counting)
- gRPC collector for high-throughput ingestion

## [0.1.0] - 2025-06-01

### Added

**Core Engine**
- OpenAI-compatible Chat Completions API (`/v1/chat/completions`)
- Governance engine with per-request token limits, daily user budgets (tokens + cost), model allowlists, and blocked pattern matching
- Inference router with cloud provider dispatch and local model fallback
- HTTP provider abstraction with streaming (SSE) support via `eventsource-stream`
- Latency tracer: Time-to-First-Token (TTFT), total latency, routing overhead
- Cost calculator with per-model pricing tables
- Fallback manager with failure tracking and automatic provider degradation

**Storage**
- SQLite embedded persistence with WAL mode and 64MB cache
- Configurable privacy filter: per-field storage controls (`store_prompt`, `store_response`, `store_cost`, `store_provider`, `store_model`, `store_latency`, `store_identity`)
- PII masking (email, phone, SSN, credit card patterns)
- Async batch sync to central collector with configurable interval and batch size
- Forward-only schema migration system

**Local Inference**
- Inference backend abstraction (`InferenceBackend` trait)
- Generic HTTP backend (compatible with Ollama, vLLM, llama.cpp server)
- CoreML backend stub (iOS/macOS)
- ONNX backend stub (Windows/Linux)
- NNAPI backend stub (Android)

**Server**
- Axum-based HTTP proxy server with CORS and request tracing
- Identity injection via `X-Duanes-*` headers
- Management endpoints: health check, daily stats, recent records, provider listing, model listing
- Standalone binary with CLI configuration

**Collector**
- Central telemetry ingestion service with `INSERT OR IGNORE` idempotency
- Aggregation APIs: overview, by-source, by-user, by-model
- SQLite-backed analytics store

**Bindings**
- C FFI bindings via `cbindgen` (static + dynamic library, 8 exported symbols)
- JNI bindings for Android (Java/Kotlin)
- UniFFI bindings for iOS (Swift)
- WebAssembly bindings via `wasm-bindgen`

**SDKs**
- Python SDK (`duanes-aigovernance` on PyPI, httpx + pydantic)
- TypeScript SDK (`duanes-aigovernance` on npm, fetch-based)
- Go SDK (`github.com/2mes4/duanes-aigovernance/sdks/go`)

**Services**
- React dashboard with real-time stats (cost, tokens, latency by user/model/source)
- Telemetry processor worker (aggregation + retention)

**Infrastructure**
- Docker multi-stage builds for server and collector
- GitHub Actions CI: lint (rustfmt + clippy), test (multi-OS), build (all targets)
- GitHub Actions release: cross-compilation, GitHub Releases, crates.io publish
- Makefile with build, test, lint, release, and Docker targets
- Cross-compilation scripts for Linux ARM64, WebAssembly, Android, iOS
