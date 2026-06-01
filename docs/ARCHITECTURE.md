# Duanes — Architecture Reference

This document is the canonical architectural reference for Duanes. It is written for contributors, integrators, and security reviewers who need to understand how the system works internally.

---

## 1. System Topology

```
 ┌─────────────────────────────────────────────────────────────────┐
 │                        EDGE LAYER                                │
 │                                                                  │
 │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐           │
 │  │ Mobile  │  │ Desktop │  │   Pod   │  │   IoT   │           │
 │  │  App    │  │  App    │  │ (K8s)   │  │  Device │           │
 │  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘           │
 │       │            │            │            │                  │
 │       ▼            ▼            ▼            ▼                  │
 │  ┌─────────────────────────────────────────────────┐           │
 │  │              Duanes Engine (Rust)                │           │
 │  │                                                  │           │
 │  │  ┌──────────┐  ┌──────────┐  ┌───────────────┐ │           │
 │  │  │ Core     │  │ Storage  │  │ Local Runner  │ │           │
 │  │  │ (gov,    │  │ (SQLite, │  │ (CoreML/ONNX/ │ │           │
 │  │  │  route)  │  │  privacy)│  │  NNAPI)       │ │           │
 │  │  └────┬─────┘  └────┬─────┘  └───────────────┘ │           │
 │  │       │              │                          │           │
 │  │       ▼              ▼                          │           │
 │  │  ┌──────────────────────────┐                   │           │
 │  │  │   Identity Injection     │                   │           │
 │  │  │   (via X-Duanes-* hdrs)  │                   │           │
 │  │  └──────────────────────────┘                   │           │
 │  └──────────────────────┬──────────────────────────┘           │
 │                         │                                        │
 └─────────────────────────┼────────────────────────────────────────┘
                           │  async batch sync (30s)
                           ▼
 ┌─────────────────────────────────────────────────────────────────┐
 │                     CONTROL LAYER                                │
 │                                                                  │
 │  ┌──────────────────┐    ┌──────────────────┐                   │
 │  │   Collector      │───▶│ Telemetry        │                   │
 │  │   (HTTP ingest)  │    │ Processor        │                   │
 │  └────────┬─────────┘    └──────────────────┘                   │
 │           │                                                      │
 │           ▼                                                      │
 │  ┌──────────────────┐    ┌──────────────────┐                   │
 │  │   Dashboard      │    │   Analytics DB   │                   │
 │  │   (React UI)     │───▶│   (SQLite /      │                   │
 │  │                  │    │    ClickHouse)   │                   │
 │  └──────────────────┘    └──────────────────┘                   │
 └─────────────────────────────────────────────────────────────────┘
```

---

## 2. Data Flow: Single Inference Request

This is the **hot path** — the route from request arrival to response return.

```
Time ──────────────────────────────────────────────────────▶

t=0μs      Request arrives at /v1/chat/completions
           │
           ▼
t~5μs      OpenAI-compat parser extracts model, messages, params
           │
           ▼
t~10μs     Identity extractor reads X-Duanes-* headers
           │
           ▼
t~15μs     Governance engine evaluates in-memory rules:
           ├── Token limit per request? ✓
           ├── Daily token budget for user? ✓
           ├── Daily cost budget for user? ✓
           ├── Model in allowlist? ✓
           └── Prompt matches blocked patterns? ✓
           │
           ├── [VIOLATION] → Return 403 immediately
           │
           ▼
t~25μs     Router selects provider:
           ├── Is model available locally? → Local Runner
           └── Otherwise → Cloud Provider (HTTP)
           │
           ▼
t~25μs     Dispatch to provider (async HTTP)
           │
           │   ... network latency (100ms-5s) ...
           │
           ▼
t~T        First token arrives → mark TTFT
           │
           ▼
t~T+Δ      Last token arrives → mark total latency
           │
           ▼
t~T+Δ+1ms  Calculate tokens, cost
           │
           ▼
t~T+Δ+2ms  Return response to client
           │
           ╔═══════════════════════════════════════════╗
           ║  EVERYTHING BELOW IS ASYNC (background)   ║
           ╚═══════════════════════════════════════════╝
           │
           ▼
           PrivacyFilter applies configured rules:
           ├── store_prompt=false? → Destroy prompt text
           ├── store_response=false? → Destroy response text
           └── mask_pii=true? → Redact emails, phones, SSNs
           │
           ▼
           SQLite INSERT (background tokio task)
           │
           ▼
           Periodic sync (every 30s):
           Batch records → POST /api/v1/ingest → Collector
           Mark synced=1 in local DB
```

**Critical invariant:** The response is returned to the client **before** storage or sync happens. The hot path is read-only with respect to the database.

---

## 3. Component Specifications

### 3.1 duanes-core

The governance and routing engine. This is the only component that touches the hot path.

| Sub-module | Responsibility | Key types |
|---|---|---|
| `openai` | Parse/serialize OpenAI Chat Completions format | `OpenAICompatLayer`, `RequestTimer` |
| `governance` | Evaluate policies in-memory | `GovernanceEngine`, `GovernanceResult` |
| `router` | Select provider or local backend, dispatch, collect metrics | `InferenceRouter`, `RouterResult` |
| `provider` | HTTP provider abstraction, streaming support | `Provider` trait, `HttpProvider`, `ProviderRegistry` |
| `latency` | High-resolution timing, cost calculation | `LatencyTracer`, `CostCalculator` |
| `fallback` | Failure tracking, provider degradation | `FallbackManager` |

**Performance budget:** Governance evaluation + routing overhead must stay under 100μs on commodity hardware.

### 3.2 duanes-storage

Configurable persistence with privacy-first defaults.

| Sub-module | Responsibility |
|---|---|
| `sqlite` | Connection pooling, CRUD operations on `inference_records` table |
| `migration` | Schema versioning. Forward-only. No down migrations. |
| `privacy` | `PrivacyFilter` applies per-field redaction based on config |
| `sync` | Background batch sync to central collector via HTTP |

**Schema (v1):**

```sql
CREATE TABLE inference_records (
    id TEXT PRIMARY KEY,
    request_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    user_id TEXT, app_id TEXT, team_id TEXT, org_id TEXT,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    execution_mode TEXT NOT NULL,  -- "cloud_proxy" | "local_inference" | "fallback"
    prompt_hash TEXT,              -- SHA-256 of prompt (always stored)
    prompt_text TEXT,              -- Full prompt (configurable)
    response_text TEXT,            -- Full response (configurable)
    prompt_tokens INTEGER, completion_tokens INTEGER, total_tokens INTEGER,
    time_to_first_token_ms INTEGER,
    total_latency_ms INTEGER,
    routing_overhead_ms INTEGER,
    prompt_cost REAL, completion_cost REAL, total_cost REAL,
    currency TEXT,
    governance_allowed BOOLEAN,
    governance_rules TEXT,         -- JSON array
    governance_violation TEXT,
    metadata TEXT,                 -- JSON
    synced BOOLEAN DEFAULT 0       -- 1 if synced to collector
);
```

### 3.3 duanes-local-runner

Platform-specific inference backends. Each backend implements:

```rust
#[async_trait]
pub trait InferenceBackend: Send + Sync {
    fn name(&self) -> &str;
    async fn inference(&self, request: InferenceRequest) -> DuanesResult<InferenceResponse>;
    fn supports_model(&self, model: &str) -> bool;
    fn list_models(&self) -> Vec<String>;
}
```

| Backend | Platform | Status | Underlying tech |
|---|---|---|---|
| `CoreMlBackend` | iOS, macOS | Stub | CoreML framework |
| `OnnxBackend` | Windows, Linux | Stub | ONNX Runtime |
| `NnapiBackend` | Android | Stub | Android NNAPI |
| `GenericHttpBackend` | All | Implemented | HTTP (Ollama, vLLM, llama.cpp server) |

### 3.4 duanes-server

Axum-based HTTP server. Stateless (all state is in the `Arc<AppState>`).

| Middleware | Function |
|---|---|
| CORS | Permissive (configure in production) |
| Tracing | `tower-http::TraceLayer` for request logging |
| Identity | Extract `X-Duanes-*` headers into `CallerIdentity` |

### 3.5 duanes-collector

Central ingestion point. Receives `SyncPayload` batches from distributed edges.

```rust
pub struct SyncPayload {
    pub source_id: String,        // Edge device identifier
    pub batch_id: Uuid,           // Deduplication key
    pub records: Vec<InferenceRecord>,
    pub timestamp: DateTime<Utc>,
}
```

**Idempotency:** Records are `INSERT OR IGNORE` on primary key. Re-sending the same batch is safe.

---

## 4. Trust and Security Model

### 4.1 Identity Injection

Duanes **does not authenticate users**. It trusts identity information injected by an upstream reverse proxy or API gateway.

```
[User] → [API Gateway (Kong/Cloudflare/Envoy)] → [Duanes Server]
                    │
                    ├── Validates JWT / OAuth / mTLS
                    ├── Injects X-Duanes-User-Id: alice@corp.com
                    ├── Injects X-Duanes-App-Id: support-bot
                    └── Injects X-Duanes-Org-Id: acme
```

**Why?** Every organization already has an auth system. We don't want to be another one. Deploy Duanes behind your existing gateway.

**Implication:** Duanes must **never** be exposed directly to the internet without an auth proxy in front.

### 4.2 Data Sensitivity Levels

| Data | Sensitivity | Default storage |
|---|---|---|
| Model name | Low | Always stored |
| Token counts | Low | Always stored |
| Cost calculation | Low | Always stored (configurable) |
| Latency metrics | Low | Always stored (configurable) |
| Provider name | Low | Always stored (configurable) |
| User identity | Medium | Stored by default (configurable) |
| Prompt hash (SHA-256) | Low | Always stored (for dedup analysis) |
| Prompt text | **High** | Configurable. Default: stored. Set `store_prompt=false` for regulated environments. |
| Response text | **High** | Configurable. Default: stored. Set `store_response=false` for regulated environments. |

### 4.3 Threat Model

| Threat | Mitigation |
|---|---|
| Prompt leakage via logs | Prompt content is never logged at any level. `tracing` calls use metadata only. |
| Prompt leakage via storage | `PrivacyFilter` with `store_prompt=false` destroys text in memory before SQLite write. |
| Prompt leakage via collector sync | Sync payload respects the same privacy flags. If `store_prompt=false`, the prompt never leaves the edge. |
| Replay attacks on collector | `INSERT OR IGNORE` on record ID. Idempotent ingestion. |
| Collector spoofing | Optional `api_key` in `CollectorConfig`. Collector verifies `Authorization: Bearer` header. |

---

## 5. Deployment Patterns

### 5.1 Single-user desktop app

```
[Desktop App (C++)] ──FFI──▶ [libduanes_c.a] ──HTTP──▶ [OpenAI]
                                  │
                                  └──▶ [SQLite local]
```

Duanes is embedded as a static library. No server process. No collector. The app reads stats via `duanes_get_stats()`.

### 5.2 Team proxy (most common)

```
[10 microservices] ──HTTP──▶ [duanes-server] ──HTTP──▶ [OpenAI / Anthropic]
                                   │
                                   ├──▶ [SQLite]
                                   └──▶ [duanes-collector] ──▶ [Dashboard]
```

All services point to `duanes-server` as their `base_url`. FinOps team watches the dashboard.

### 5.3 Enterprise edge fleet

```
[10,000 mobile devices]
     │  (duanes embedded via JNI/UniFFI)
     │
     ├──▶ [Local inference via CoreML/NNAPI]
     ├──▶ [Cloud fallback via OpenAI]
     ├──▶ [SQLite local]
     └──▶ Async sync (batch every 5 min)
              │
              ▼
         [duanes-collector (load-balanced)]
              │
              ▼
         [Dashboard] (FinOps + Security teams)
```

### 5.4 Regulated environment (HIPAA / GDPR)

```toml
[storage.privacy]
store_prompt = false          # Patient data never persisted
store_response = false        # AI output never persisted
store_identity = true         # But we know WHO called
store_cost = true             # And WHAT it cost
mask_pii = true               # Extra safety: redact anything that looks like PII
```

---

## 6. Cross-Compilation Targets

| Target | Triple | Build command |
|---|---|---|
| Linux x86_64 | `x86_64-unknown-linux-gnu` | `cargo build --release --target x86_64-unknown-linux-gnu` |
| Linux ARM64 | `aarch64-unknown-linux-gnu` | `cargo build --release --target aarch64-unknown-linux-gnu` |
| macOS x86_64 | `x86_64-apple-darwin` | `cargo build --release --target x86_64-apple-darwin` |
| macOS ARM64 | `aarch64-apple-darwin` | `cargo build --release --target aarch64-apple-darwin` |
| Windows x86_64 | `x86_64-pc-windows-msvc` | `cargo build --release --target x86_64-pc-windows-msvc` |
| Android ARM64 | `aarch64-linux-android` | `cargo ndk -t arm64-v8a build` |
| iOS ARM64 | `aarch64-apple-ios` | `cargo lipo --release -p duanes-ios` |
| WebAssembly | `wasm32-unknown-unknown` | `wasm-pack build bindings/duanes-wasm` |

---

## 7. Design Decisions Log

### Why Rust?

Microsecond routing overhead. Cross-compilation to 8+ targets from one codebase. Memory safety without GC pauses. The `serde` ecosystem makes OpenAI-format parsing trivial.

### Why SQLite and not PostgreSQL?

Duanes runs on the edge. Phones don't have PostgreSQL. SQLite is embedded, zero-config, and handles 100K+ writes/second in WAL mode. The collector can use ClickHouse for enterprise scale.

### Why identity trust (not auth)?

Every organization has different auth: SAML, OIDC, mTLS, API keys. Building auth into Duanes would mean re-implementing what Kong/Envoy/Cloudflare already do. Instead, Duanes integrates with whatever you already have.

### Why OpenAI format as the universal interface?

It's the de facto standard. Every AI provider offers an OpenAI-compatible mode (Anthropic via proxy, Gemini via proxy, vLLM, Ollama, llama.cpp). By being OpenAI-compatible, Duanes is automatically compatible with everything.

### Why not just use LangChain / LiteLLM?

LangChain is a framework for building AI apps. LiteLLM is a Python proxy. Neither runs on mobile devices, neither compiles to WebAssembly, neither adds microsecond overhead, and neither has a governance-first design.

---

*This document is maintained by the 2mes4 team. For questions, open a [Discussion](https://github.com/2mes4/duanes-aigovernance/discussions).*
