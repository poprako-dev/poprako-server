---
name: tracing-usage-spec
description: |
  Specifies where #[tracing::instrument] should and should NOT be placed in
  poprako-r. Use whenever adding or reviewing #[tracing::instrument] annotations
  in any Rust source file.
---

# Tracing Instrument Usage Specification

`#[tracing::instrument]` creates a span for every invocation of the decorated
function. This has a runtime cost, so it must only be placed where the
observability benefit outweighs that cost. Two categories of functions must
**never** carry `#[tracing::instrument]`.

## Rule 1: No instrument on constructors

Do **NOT** place `#[tracing::instrument]` on `new()`, constructor functions,
ID generators, or any function whose primary purpose is creating a value.

**Why**: Constructors are trivial glue code — they assign fields and return.
They never fail, never perform I/O, and are called with extremely high
frequency (every request path hits multiple constructors). Instrumenting them
floods traces with noise and wastes CPU on span creation/destruction for
operations that are not diagnostically useful.

**Examples of prohibited functions:**
- `fn new(...)` — any type's constructor
- `fn generate_id()` — aggregate ID generation
- `fn generate_avatar_key()` — derived key generation
- `fn from(...)` — `From`/`Into` conversions
- `impl Default for ...` — `default()` constructors

**Do NOT:**
```rust
impl UserForm {
    #[tracing::instrument]           // BAD — constructor
    pub fn new(qid: String, nickname: String, password: String) -> Self { ... }
}

impl UserAggr {
    #[tracing::instrument]           // BAD — ID generator
    pub fn generate_id() -> String { ... }
}
```

**Do:**
```rust
impl UserForm {
    pub fn new(qid: String, nickname: String, password: String) -> Self { ... }
}

impl UserAggr {
    pub fn generate_id() -> String { ... }
}
```

## Rule 2: No instrument on pure logic functions

Do **NOT** place `#[tracing::instrument]` on any function with a body
(implementation) under `src/domain/`. This covers:

- Domain model methods (aggregate impl blocks in `src/domain/model/aggregate/`)
- Domain value object methods (`src/domain/model/value/`)
- Domain event constructors / helpers (`src/domain/model/event/`)
- Domain service functions (`src/domain/svc/`)
- Actor definitions (`src/domain/actor/`)

**Why**: Domain functions are pure business logic. They do not perform I/O,
do not acquire locks, and are called synchronously inside the request path.
Span boundaries should be drawn at I/O and orchestration boundaries
(usecase layer, infrastructure layer) — not inside the domain model. Adding
spans to domain functions creates excessive trace depth without helping
localize failures, because domain failures are always propagated through and
caught at the usecase/infrastructure boundary where the span already exists.

**Do NOT:**
```rust
// src/domain/model/aggregate/user.rs
impl UserCredential {
    #[tracing::instrument]           // BAD — pure logic, no I/O
    pub fn verify_password(&self, password: &str) -> bool { ... }
}

// src/domain/svc/some_service.rs
#[tracing::instrument]               // BAD — domain service
pub fn validate_something(input: &str) -> Result<(), Error> { ... }
```

**Do:**
```rust
// Domain functions carry no instrument annotation at all.
impl UserCredential {
    pub fn verify_password(&self, password: &str) -> bool { ... }
}
```

## Where instrument IS appropriate

`#[tracing::instrument]` is valuable on:

| Layer | Rationale |
|-------|-----------|
| `src/usecase/` | Orchestration boundaries — spans mark the start of a use-case, capturing the full request lifecycle |
| `src/infrastructure/query/` | Database I/O — spans let you trace individual queries, their duration, and parameters |
| `src/infrastructure/external/` | External service calls (OSS, token generation, etc.) — spans isolate external latency |
| `src/api/` | HTTP handler entry points — spans mark request boundaries |

These layers perform I/O, can fail independently, and benefit from
per-operation observability.

## Relationship to thirdparty-macro-usage-spec

The `thirdparty-macro-usage-spec` skill governs **how** macros are imported
and invoked:
- `#[instrument]` uses `use tracing::instrument;` + bare name.
- `tracing::error!`, `tracing::warn!`, `tracing::info!`, `tracing::debug!`
  must use fully qualified paths at the call site — never imported into scope.

This skill (`tracing-usage-spec`) governs **where** `#[instrument]`
belongs. Both apply when working with `#[instrument]` and tracing event
macros.
