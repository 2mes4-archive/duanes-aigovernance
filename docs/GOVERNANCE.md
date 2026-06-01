# Duanes Governance Model

This document describes the governance policies that Duanes can enforce, how they are configured, and how they are evaluated at runtime.

---

## Overview

Governance in Duanes is the set of rules that determine whether an inference request is **allowed to proceed**. Rules are evaluated **synchronously** in the hot path, in memory, with microsecond overhead.

If a rule is violated, the request is rejected with an HTTP 403 and the violation is logged.

---

## Policy Types

### 1. Token Limit per Request

Rejects requests that would exceed a token threshold.

```toml
[governance]
max_tokens_per_request = 128000
```

**Evaluation:** Prompt tokens are estimated via character count (4 chars ≈ 1 token). If `max_tokens` is specified in the request, the sum is checked.

**Use case:** Prevent a single user from submitting a 1M-token document that costs $30 in one call.

### 2. Daily Token Budget per User

Rejects requests when a user exceeds their daily token allocation.

```toml
[governance]
max_tokens_per_user_daily = 1000000
```

**Evaluation:** The governance engine tracks cumulative daily usage per `X-Duanes-User-Id` in a `DashMap`. At midnight (configurable), the counters reset.

**Use case:** Give every team member a fair-use allowance of 1M tokens/day.

### 3. Daily Cost Budget per User

Rejects requests when a user exceeds their daily spend limit.

```toml
[governance]
max_cost_per_user_daily = 50.0
```

**Evaluation:** Same as token budget, but tracks accumulated dollar cost based on the model pricing table.

**Use case:** Cap individual spend at $50/day. Prevents accidental runaway costs from loops or bugs.

### 4. Model Allowlist

Restricts which models can be used.

```toml
[governance]
allowed_models = ["gpt-4o-mini", "gpt-4o", "claude-3-5-sonnet"]
```

**Evaluation:** Substring match against the requested model name. If the request is for `gpt-4-32k` and it's not in the allowlist, the request is rejected.

**Use case:** Prevent developers from using expensive models (e.g., GPT-4 Turbo) without approval.

### 5. Blocked Patterns

Rejects prompts containing sensitive or forbidden content.

```toml
[governance]
blocked_patterns = [
    "credit card number",
    "social security",
    "internal API key",
    "CONFIDENTIAL"
]
```

**Evaluation:** Case-insensitive substring match against the concatenated prompt text. If any pattern matches, the request is rejected.

**Use case:** Prevent users from accidentally sending PII or secrets to external AI providers.

---

## Policy Evaluation Order

Policies are evaluated in a fixed order. The first violation short-circuits the rest:

```
1. Governance enabled?       → If no, skip all checks
2. Token limit per request?  → If violated, reject
3. Model in allowlist?       → If not, reject
4. Blocked patterns?         → If matched, reject
5. Daily token budget?       → If exceeded, reject
6. Daily cost budget?        → If exceeded, reject
7. All passed → Allow
```

Each evaluation is recorded in `GovernanceResult.rules_evaluated` for audit purposes, regardless of outcome.

---

## Runtime Behavior

### When a request is rejected

```http
HTTP/1.1 403 Forbidden
Content-Type: text/plain

Governance policy violated: User 'alice' exceeds daily cost budget: $52.30 used of $50.00
```

The rejection is **still recorded** in the database (with `governance_allowed = false` and the violation message). This allows administrators to see attempted violations in the dashboard.

### When governance is disabled

```toml
[governance]
enabled = false
```

No rules are evaluated. All requests pass through. The `GovernanceResult` records `["governance_disabled"]` in the rules evaluated list. This is useful for development environments.

---

## Cost Calculation

Costs are calculated based on a pricing table. The table is defined per-provider in the provider configuration:

```toml
[[providers]]
name = "openai"
base_url = "https://api.openai.com"
api_key = "sk-..."

  [[providers.models]]
  model_id = "gpt-4o"
  prompt_cost_per_million = 2.50
  completion_cost_per_million = 10.00

  [[providers.models]]
  model_id = "gpt-4o-mini"
  prompt_cost_per_million = 0.15
  completion_cost_per_million = 0.60
```

**Formula:**

```
prompt_cost     = (prompt_tokens     / 1,000,000) × prompt_cost_per_million
completion_cost = (completion_tokens / 1,000,000) × completion_cost_per_million
total_cost      = prompt_cost + completion_cost
```

Token counts are estimated locally using a character-based heuristic (4 characters ≈ 1 token). For exact billing reconciliation, use the provider's own usage reports.

---

## Budget Reset

Daily budgets reset via a cron-like mechanism. The simplest approach is to call:

```rust
governance_engine.reset_daily().await;
```

This clears all `DashMap` counters. In production, this should be triggered by a cron job or the telemetry processor at `00:00 UTC`.

---

## Future Governance Features (Roadmap)

| Feature | Description | Status |
|---|---|---|
| **Per-app budgets** | Distinct budgets per `X-Duanes-App-Id` | Planned |
| **Rate limiting** | N requests/minute per user | Planned |
| **Prompt rewriting** | Auto-redact secrets before forwarding | Planned |
| **Model tiering** | Auto-downgrade to cheaper model when budget is near limit | Planned |
| **Time-based policies** | Only allow expensive models during business hours | Planned |
| **Custom rule engine** | WASM-based user-defined rules | Future |

---

*For implementation details, see `crates/duanes-core/src/governance.rs`.*
