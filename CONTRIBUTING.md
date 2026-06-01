# Contributing to Duanes

Thank you for your interest in contributing to Duanes. This document is the single source of truth for how we build, test, and release the project. Read it carefully before opening a pull request.

---

## Table of Contents

1. [Project Philosophy](#1-project-philosophy)
2. [What We Accept](#2-what-we-accept)
3. [What We Reject](#3-what-we-reject)
4. [Development Environment](#4-development-environment)
5. [Code Standards](#5-code-standards)
6. [Architecture Constraints](#6-architecture-constraints)
7. [Testing Policy](#7-testing-policy)
8. [Pull Request Process](#8-pull-request-process)
9. [Release Process](#9-release-process)
10. [Communication Channels](#10-communication-channels)

---

## 1. Project Philosophy

Duanes is built on three non-negotiable principles:

### Edge-first, not cloud-first

Duanes runs on the device where the AI call originates — a phone, a laptop, a Raspberry Pi, a Kubernetes pod. The cloud is for aggregation, not for the hot path. Every design decision must answer: **"Does this work when the network is slow or absent?"**

### Zero-latency routing

The governance and routing layer adds **microseconds**, not milliseconds. If your contribution increases routing overhead beyond 1ms on commodity hardware, it will be rejected. Storage, sync, and telemetry happen **asynchronously** and must never block the inference path.

### OpenAI-compatible or nothing

The API surface is the OpenAI Chat Completions spec. We do not invent new endpoints, new parameters, or new response formats. If the OpenAI SDK in any language cannot talk to Duanes without modification, that is a bug.

---

## 2. What We Accept

- **Bug fixes** — Always welcome. Include a failing test case.
- **Performance improvements** — Benchmark before and after. Include numbers.
- **New inference backends** — ONNX, CoreML, NNAPI, vLLM, llama.cpp, Ollama. These must implement the `InferenceBackend` trait and live behind a feature flag.
- **New SDK bindings** — Ruby, Java, Rust, Swift, Kotlin. Each lives in `sdks/<language>/` and wraps the HTTP API or the C FFI.
- **Dashboard improvements** — Charts, filters, alerts. React + Recharts.
- **Documentation** — Architecture docs, tutorials, integration guides.
- **Privacy enhancements** — New PII detection patterns, encryption at rest, data minimization.

---

## 3. What We Reject

- **Cloud-only features.** Duanes is not a SaaS. Do not submit changes that require a cloud backend to function. The core engine must work fully offline.
- **Breaking API changes.** The OpenAI-compatible API is a contract. Breaking it breaks every user. If you need to add something, add a new optional endpoint under `/api/v1/`.
- **Authentication / authorization in core.** Duanes trusts upstream identity injection. Do not add JWT validation, OAuth flows, or API key management to the core engine. That belongs in the proxy layer outside Duanes.
- **Dependency bloat.** Every new crate dependency must be justified. We compile for WebAssembly, Android, iOS, and embedded Linux. Heavy deps break cross-compilation.
- **Blocking I/O on the hot path.** Storage and network calls in the request handling path must be async and non-blocking. Use the existing `tokio` runtime.
- **Telemetry that crosses the trust boundary.** The collector receives what edges send. Do not add "phone home" telemetry to the core engine.

---

## 4. Development Environment

### Prerequisites

| Tool | Version | Required for |
|---|---|---|
| Rust | 1.85+ (stable) | Core engine, all crates |
| Node.js | 20+ | Dashboard, TypeScript SDK |
| Python | 3.9+ | Python SDK |
| Go | 1.22+ | Go SDK |
| wasm-pack | latest | WebAssembly bindings |

### Setup

```bash
git clone https://github.com/2mes4/duanes-aigovernance.git
cd duanes-aigovernance
./scripts/setup-dev.sh
```

This installs:
- Rust toolchain with all cross-compilation targets
- `cargo-audit`, `cargo-deny`, `wasm-pack`
- Node.js dependencies for the dashboard and TypeScript SDK
- Python dev dependencies

### Build

```bash
# Build everything
make build

# Build specific components
make build-server
make build-collector
make build-c-bindings
make build-wasm

# Cross-compile for a specific target
./scripts/cross-build.sh aarch64-unknown-linux-gnu
```

### Run locally

```bash
# Terminal 1: Start the collector
./target/release/duanes-collector --port 9090

# Terminal 2: Start the server
./target/release/duanes-server --config config/duanes.example.toml --port 8080

# Terminal 3: Start the dashboard
make dashboard-dev
```

---

## 5. Code Standards

### Rust

```bash
# Format check
cargo fmt --check

# Lint (zero warnings allowed)
cargo clippy --workspace --all-targets -- -D warnings

# Test
cargo test --workspace
```

**Conventions:**

- Use `thiserror` for library errors, `anyhow` for binary entry points.
- Async everywhere for I/O. `tokio` is the runtime. Use `parking_lot::Mutex` for sync contexts and `tokio::sync::RwLock` for async contexts.
- No `unwrap()` or `expect()` in library code outside of tests. Use `?` with proper error types.
- Document all `pub` items with `///` doc comments.
- Edition 2024: use `#[unsafe(no_mangle)]` for FFI exports.

### Python (SDK)

```bash
cd sdks/python
ruff check src/
mypy src/
```

- Type hints required on all public functions.
- `pydantic` for data models.
- `httpx` for HTTP (sync client, not async — keeps it simple).

### TypeScript (SDK + Dashboard)

```bash
cd sdks/typescript && npm run lint
cd services/dashboard && npm run lint
```

- Strict TypeScript (`"strict": true` in `tsconfig.json`).
- `fetch` API for HTTP. No axios.
- Functional React with hooks. No class components.

### Go (SDK)

- Idiomatic Go. `gofmt` + `go vet`.
- `context.Context` on all public functions that do I/O.
- Interfaces for mockability.

---

## 6. Architecture Constraints

### Hot path rules

The hot path is: **request arrives → governance check → provider dispatch → response returned**. Everything in this path is measured in microseconds.

| Rule | Why |
|---|---|
| No database writes on the hot path | SQLite writes go to a background tokio task |
| No HTTP calls to the collector on the hot path | Sync is async + batched |
| No regex compilation on the hot path | Compile patterns at startup |
| No allocations in the rule evaluator | Pre-allocate buffers at init |

If your change touches `crates/duanes-core/src/router.rs` or `crates/duanes-core/src/governance.rs`, benchmark it:

```bash
cargo bench -p duanes-core
```

### Privacy rules

| Rule | Why |
|---|---|
| All storage goes through `PrivacyFilter` | One chokepoint for data minimization |
| `store_prompt: false` means destroy in memory immediately | Not "store but mark as hidden" |
| PII masking patterns must be opt-in | Don't alter data without explicit config |
| Never log prompt content at any log level | Logs are often shipped to third parties |

### FFI rules

| Rule | Why |
|---|---|
| All C functions return ownership to the caller | Use `CString::into_raw` / `free_string` pairs |
| No panics across FFI | Catch with `catch_unwind` or return null |
| Tokio runtime is created once, reused | `OnceLock<Runtime>` in the C bindings |
| Header generation via `cbindgen` | Don't hand-write `duanes.h` |

---

## 7. Testing Policy

### What must be tested

| Component | Test type | Tool |
|---|---|---|
| `duanes-common` | Unit tests | `#[test]` |
| `duanes-core` | Unit + integration | `#[test]` + `#[tokio::test]` |
| `duanes-storage` | Integration (real SQLite) | `#[tokio::test]` |
| `duanes-server` | HTTP integration | `reqwest` in tests |
| SDKs | Unit + contract tests | pytest / vitest / go test |

### Test naming

```rust
#[tokio::test]
async fn governance_rejects_request_exceeding_token_limit() { ... }

#[tokio::test]
async fn storage_does_not_persist_prompt_when_disabled() { ... }
```

Describe **what** the test verifies, not **how**.

### CI requirements

All PRs must pass:

1. `cargo fmt --check` — formatting
2. `cargo clippy -- -D warnings` — lint
3. `cargo test --workspace` — all tests
4. `cargo audit` — no known vulnerabilities
5. SDK-specific lint + build (Python, TypeScript, Go)

---

## 8. Pull Request Process

### Before you start

1. Check [existing issues](https://github.com/2mes4/duanes-aigovernance/issues) and [discussions](https://github.com/2mes4/duanes-aigovernance/discussions).
2. For features, open an issue first. Don't invest time on a PR that might be rejected.
3. For bugs, include a minimal reproduction case.

### Branch naming

```
feat/<short-description>      # New features
fix/<short-description>       # Bug fixes
docs/<short-description>      # Documentation only
chore/<short-description>     # Build, CI, deps
```

### Commit messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(core): add per-model rate limiting
fix(storage): respect store_prompt=false in async sync
docs(arch): update data flow diagram
chore(deps): bump axum to 0.8.9
```

### PR checklist

- [ ] Branch is up to date with `main`
- [ ] `cargo fmt` has been run
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo test --workspace` passes
- [ ] New code has tests
- [ ] Public API changes are documented
- [ ] No secrets, API keys, or credentials in the diff
- [ ] Commit messages follow conventional commits

### Review criteria

Maintainers review for:

1. **Correctness** — Does it do what the PR says?
2. **Performance** — Does it keep the hot path fast?
3. **Privacy** — Does it respect the privacy filter contract?
4. **API compatibility** — Does it break existing users?
5. **Cross-platform** — Does it compile for all targets?
6. **Test coverage** — Are edge cases covered?

### Response times

- Initial review: within 3 business days
- Follow-up reviews: within 2 business days
- Merge: after all review comments are resolved and CI is green

---

## 9. Release Process

Releases are managed by the 2mes4 team.

### Versioning

We follow [Semantic Versioning](https://semver.org/):

- **MAJOR:** Breaking API changes (the OpenAI-compatible interface changes)
- **MINOR:** New features, new backends, new SDK methods (backwards-compatible)
- **PATCH:** Bug fixes, performance improvements, dependency updates

### Release artifacts

Each release publishes:

| Artifact | Where |
|---|---|
| `duanes-server` binaries (Linux/macOS/Windows) | GitHub Releases |
| `duanes-collector` binaries | GitHub Releases |
| `libduanes_c.a` / `libduanes_c.so` / `duanes.h` | GitHub Releases |
| `duanes_wasm_bg.wasm` | GitHub Releases |
| Rust crates | crates.io |
| Python SDK | PyPI (`duanes-aigovernance`) |
| TypeScript SDK | npm (`duanes-aigovernance`) |
| Docker images | `ghcr.io/2mes4/duanes-server`, `ghcr.io/2mes4/duanes-collector` |

### Changelog

Every release updates [CHANGELOG.md](CHANGELOG.md) under the `## [Unreleased]` section. PRs that add user-facing changes should include a changelog entry.

---

## 10. Communication Channels

| Channel | Use for |
|---|---|
| [GitHub Issues](https://github.com/2mes4/duanes-aigovernance/issues) | Bug reports, feature requests |
| [GitHub Discussions](https://github.com/2mes4/duanes-aigovernance/discussions) | Questions, architecture debate, show & tell |
| Pull Request comments | Code-specific discussion |

### Reporting security vulnerabilities

**Do not open a public issue.** Email `security@2mes4.com` with:

1. Description of the vulnerability
2. Steps to reproduce
3. Affected versions
4. Suggested fix (optional)

We respond within 48 hours and coordinate disclosure.

---

## License

By contributing to Duanes, you agree that your contributions are licensed under the [Apache License 2.0](LICENSE).

---

*This document is maintained by the 2mes4 team. Last updated: 2025-06-01.*
