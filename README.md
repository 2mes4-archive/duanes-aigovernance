<div align="center">

# Duanes

### The AI governance framework that disappears into your stack

**Drop-in replacement for the OpenAI SDK.** One URL change. Total visibility, total control, zero latency penalty.

[![CI](https://github.com/2mes4/duanes-aigovernance/actions/workflows/ci.yml/badge.svg)](https://github.com/2mes4/duanes-aigovernance/actions/workflows/ci.yml)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Rust](https://img.shields.io/badge/Rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)
[![Built by 2mes4](https://img.shields.io/badge/Built%20by-2mes4-6366f1.svg)](https://github.com/2mes4)

</div>

---

## Stop flying blind with AI

Every AI call your team makes is a black box. You don't know who spent what, which model was used, what data was sent, or why your cloud bill tripled last month.

**Duanes fixes this in 30 seconds.**

Change one line of code. Every AI call — whether it hits OpenAI, Anthropic, or a local model on a phone — flows through Duanes. You get **cost tracking, latency measurement, governance policies, and audit logs** without touching a single line of business logic.

### Why Duanes?

| | Without Duanes | With Duanes |
|---|---|---|
| **Cost visibility** | End-of-month surprise bill | Real-time per-user, per-app, per-model |
| **Vendor lock-in** | Tied to one provider | Swap providers with one config change |
| **Governance** | "Please don't use GPT-4 for that" | Enforced budgets, rate limits, model allowlists |
| **Compliance** | No audit trail | Every call logged, PII-maskable, exportable |
| **Offline AI** | Impossible | Local models on-device via CoreML/ONNX/NNAPI |
| **Performance** | Proxy overhead | Rust-native, microsecond routing overhead |

---

## Quick Start

### 1. Install

```bash
# From source (50-second compile)
git clone https://github.com/2mes4/duanes-aigovernance.git
cd duanes-aigovernance
make build
```

### 2. Configure

Create `duanes.toml`:

```toml
[engine]
default_provider = "openai"

[storage]
backend = "sqlite"
path = "data/duanes.db"

[storage.privacy]
store_prompt = false        # Don't store sensitive data
store_response = false
store_cost = true           # But track every cent

[governance]
enabled = true
max_cost_per_user_daily = 50.0
allowed_models = ["gpt-4o-mini", "gpt-4o"]
```

### 3. Run

```bash
./duanes-server --config duanes.toml --port 8080
```

### 4. Change one line in your app

**Before:**
```python
client = OpenAI(base_url="https://api.openai.com/v1")
```

**After:**
```python
client = OpenAI(base_url="http://localhost:8080/v1")
```

That's it. Every call now flows through Duanes. Your app code doesn't change.

---

## How it works

```
 Your App                    Duanes (Rust, <1ms overhead)              AI Provider
 ┌──────────┐               ┌──────────────────────────┐              ┌──────────┐
 │ openai   │── request ──▶ │ ① Governance rules       │── proxy ──▶ │  OpenAI  │
 │ SDK      │               │ ② Cost calculation        │              │  Claude  │
 │ (any     │               │ ③ Latency tracing         │              │  Gemini  │
 │  language)│◀─ response ─ │ ④ Configurable storage    │◀─ response ─ │  Local   │
 └──────────┘               │ ⑤ Async sync to collector │              └──────────┘
                            └──────────────────────────┘
                                         │
                                         ▼ async
                               ┌──────────────────┐
                               │ Central Collector │
                               │ + Dashboard       │
                               └──────────────────┘
```

**Key design principles:**

- **Zero-latency routing.** Rust-native, in-memory rule evaluation. Routing overhead is microseconds.
- **Privacy-first.** You decide what gets stored. Destroy prompts immediately, keep only costs and metrics.
- **Identity by trust.** Your API gateway injects user identity via headers. Duanes trusts it and tracks it.
- **Local-first.** Runs on edge devices — phones, desktops, IoT — not just servers.
- **OpenAI-compatible.** If your SDK speaks OpenAI, Duanes works. No code changes. Ever.

---

## Usage examples

### Python

```python
from duanes_aigovernance import DuanesClient
from duanes_aigovernance.client import ChatMessage

client = DuanesClient(
    base_url="http://localhost:8080",
    user_id="alice@company.com",
    app_id="support-bot",
)

response = client.chat_completions(
    model="gpt-4o-mini",
    messages=[ChatMessage(role="user", content="Summarize this ticket")],
)

# Check your spend in real-time
stats = client.daily_stats()
print(f"Spent ${stats['total_cost']:.2f} today across {stats['total_requests']} calls")
```

### TypeScript / Node.js

```typescript
import { DuanesClient } from "duanes-aigovernance";

const client = new DuanesClient({
  baseUrl: "http://localhost:8080",
  userId: "alice@company.com",
  appId: "support-bot",
});

const response = await client.chatCompletions({
  model: "gpt-4o-mini",
  messages: [{ role: "user", content: "Summarize this ticket" }],
});
```

### Go

```go
client := duanes.NewClient("http://localhost:8080",
    duanes.WithUserID("alice@company.com"),
    duanes.WithAppID("support-bot"),
)

resp, err := client.ChatCompletions(ctx, duanes.ChatCompletionRequest{
    Model: "gpt-4o-mini",
    Messages: []duanes.ChatMessage{
        {Role: "user", Content: "Summarize this ticket"},
    },
})
```

### C / C++ (embedded)

```c
#include "duanes.h"

DuanesEngine* engine = duanes_init("duanes.toml");

const char* response = duanes_inference(engine,
    "{\"model\":\"gpt-4o-mini\","
    "\"messages\":[{\"role\":\"user\",\"content\":\"Hello\"}]}",
    "alice@company.com"
);

printf("%s\n", response);
duanes_destroy(engine);
```

### Use the OpenAI SDK directly (any language)

```python
# Just change the base URL. Your existing OpenAI SDK works as-is.
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:8080/v1",
    api_key="anything"  # Duanes doesn't check this; your proxy does
)

response = client.chat.completions.create(
    model="gpt-4o-mini",
    messages=[{"role": "user", "content": "Hello!"}],
)
```

---

## Architecture

Duanes is split into two planes:

### Data Plane (hot path, microseconds)

| Component | What it does |
|---|---|
| **duanes-core** | Intercepts OpenAI-format requests, evaluates governance rules, routes to provider |
| **duanes-local-runner** | Executes models on-device via CoreML (iOS/macOS), ONNX (Windows/Linux), NNAPI (Android) |
| **duanes-storage** | Stores what you configure (costs always, prompts optionally) in embedded SQLite |

### Control Plane (async, background)

| Component | What it does |
|---|---|
| **duanes-server** | Standalone HTTP proxy. Point your apps here. |
| **duanes-collector** | Central telemetry ingestion. Receives data from distributed edges. |
| **dashboard** | React UI showing costs, latency, usage by user/model/source. |
| **telemetry-processor** | Background worker for aggregation and data retention. |

### Bindings (embed Duanes in your app)

| Binding | Platforms | Use case |
|---|---|---|
| **duanes-c** | All (C ABI) | Embed in any language: C++, Rust, Zig, Go, Python via ctypes |
| **duanes-android** | Android (JNI) | Native Kotlin/Java apps |
| **duanes-ios** | iOS/macOS (UniFFI) | Native Swift apps |
| **duanes-wasm** | Browser / Edge workers | WebAssembly environments |

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the full technical deep-dive.

---

## Configuration reference

```toml
[engine]
default_provider = "openai"          # Which provider to use by default
request_timeout_secs = 120            # Timeout for upstream AI calls
max_retries = 3                       # Retry on provider failure
retry_delay_ms = 1000

[storage]
backend = "sqlite"                    # "sqlite" or "in_memory"
path = "data/duanes.db"               # Where to store the local database

[storage.privacy]
store_prompt = true                   # Store the full prompt text?
store_response = true                 # Store the full response text?
store_cost = true                     # Store cost calculations?
store_provider = true                 # Store which provider was used?
store_model = true                    # Store which model was used?
store_latency = true                  # Store timing data?
store_identity = true                 # Store user identity?
mask_pii = false                      # Auto-redact emails, phones, SSNs?

[governance]
enabled = true                        # Enable policy enforcement
max_tokens_per_request = 128000       # Reject requests above this
max_tokens_per_user_daily = 1000000   # Daily token budget per user
max_cost_per_user_daily = 50.0        # Daily cost budget per user ($)
allowed_models = ["gpt-4o", "gpt-4o-mini"]  # Model allowlist
blocked_patterns = []                 # Block prompts matching these strings

[collector]                           # Optional: central telemetry
endpoint = "http://collector:9090"
sync_interval_secs = 30               # How often to sync
batch_size = 100                      # Records per batch

[local_runner]                        # Optional: on-device models
enabled = false
models_path = "./models"
default_backend = "auto"              # "coreml", "onnx", "nnapi", "auto"
max_concurrent_inferences = 2
```

---

## API Reference

### OpenAI-compatible (drop-in replacement)

| Method | Endpoint | Description |
|---|---|---|
| `POST` | `/v1/chat/completions` | Chat completions (governed, tracked) |
| `GET` | `/v1/models` | List available models |

### Identity Injection (via headers)

| Header | Required | Description |
|---|---|---|
| `X-Duanes-User-Id` | Recommended | Who is making this call? |
| `X-Duanes-App-Id` | Optional | Which application? |
| `X-Duanes-Team-Id` | Optional | Which team? |
| `X-Duanes-Org-Id` | Optional | Which organization? |

> **Trust model:** Identity is injected by your upstream API gateway (Kong, Cloudflare, Envoy, etc.). Duanes trusts the headers and does not perform its own authentication. See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the security model.

### Management & Stats

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/health` | Health check |
| `GET` | `/api/v1/stats/daily` | Today's aggregated stats |
| `GET` | `/api/v1/stats/recent` | Last 50 inference records |
| `GET` | `/api/v1/providers` | List configured providers |

### Collector API

| Method | Endpoint | Description |
|---|---|---|
| `POST` | `/api/v1/ingest` | Ingest a batch of telemetry records |
| `GET` | `/api/v1/stats/overview` | Global aggregated stats |
| `GET` | `/api/v1/stats/by-source` | Stats grouped by edge source |
| `GET` | `/api/v1/stats/by-user` | Stats grouped by user |
| `GET` | `/api/v1/stats/by-model` | Stats grouped by model |

---

## Deployment

### Local development

```bash
make build
./build/duanes-server --config config/duanes.example.toml
./build/duanes-collector --port 9090
```

### Docker

```bash
cd docker
docker compose up -d
```

### Cross-compilation for edge targets

```bash
# Linux ARM64 (Raspberry Pi, edge servers)
./scripts/cross-build.sh aarch64-unknown-linux-gnu

# WebAssembly (browser, Cloudflare Workers)
wasm-pack build bindings/duanes-wasm --target web

# Android
make build-android

# iOS
make build-ios
```

---

## Project structure

```
duanes-aigovernance/
├── crates/                   # Rust core (the engine)
│   ├── duanes-common/            # Shared types, config, errors
│   ├── duanes-core/              # Governance, routing, latency tracing
│   ├── duanes-storage/           # SQLite + privacy filters + async sync
│   ├── duanes-local-runner/      # CoreML, ONNX, NNAPI backends
│   ├── duanes-server/            # HTTP proxy server
│   └── duanes-collector/         # Central telemetry service
├── bindings/                 # FFI for native platforms
│   ├── duanes-c/                 # C headers + static/dynamic lib
│   ├── duanes-android/           # JNI (Java/Kotlin)
│   ├── duanes-ios/               # UniFFI (Swift)
│   └── duanes-wasm/              # WebAssembly
├── sdks/                     # Idiomatic client libraries
│   ├── python/                   # pip install duanes-aigovernance
│   ├── typescript/               # npm install duanes-aigovernance
│   └── go/                       # go get github.com/2mes4/duanes-aigovernance/sdks/go
├── services/                 # Operational layer
│   ├── dashboard/                # React monitoring UI
│   └── telemetry-processor/      # Aggregation + retention worker
├── docs/                     # Architecture & governance docs
├── docker/                   # Dockerfile + docker-compose
├── scripts/                  # Build + setup scripts
├── config/                   # Reference configurations
└── .github/                  # CI/CD pipelines
```

---

## Roadmap

- [x] Core governance engine (Rust)
- [x] SQLite persistence with privacy filters
- [x] HTTP proxy server (OpenAI-compatible)
- [x] Central telemetry collector
- [x] C FFI bindings
- [x] Python / TypeScript / Go SDKs
- [x] React dashboard
- [ ] **Streaming support** (SSE passthrough with token counting)
- [ ] **gRPC collector** for high-throughput ingestion
- [ ] **CoreML backend** (iOS/macOS on-device inference)
- [ ] **ONNX backend** (Windows/Linux on-device inference)
- [ ] **NNAPI backend** (Android on-device inference)
- [ ] **Prompt rewriting** (redact secrets before they reach the provider)
- [ ] **Rate limiting** (per-user, per-app, per-model)
- [ ] **Multi-region collector** federation
- [ ] **ClickHouse** adapter for enterprise-scale analytics

---

## Community & Support

- **Issues:** [GitHub Issues](https://github.com/2mes4/duanes-aigovernance/issues)
- **Discussions:** [GitHub Discussions](https://github.com/2mes4/duanes-aigovernance/discussions)
- **Contributing:** See [CONTRIBUTING.md](CONTRIBUTING.md)

---

## License

Apache License 2.0 — see [LICENSE](LICENSE). Built and maintained by [**2mes4**](https://github.com/2mes4).
