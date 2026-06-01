---
name: error-handling-spec
description: |
  Describes poprako-r's error definition and handling philosophy across all
  layers (domain, usecase, infrastructure, api). Use whenever adding or
  modifying error types, constructing errors, adding trace_* calls, placing
  #[tracing::instrument], or introducing new i18n error keys.
---

# Poprako-r Error Handling Specification

## Core Philosophy

All errors in poprako-r fall into exactly one of two categories:

| Category | Variant | Audience | trace level | Contains location? |
|----------|---------|----------|-------------|---------------------|
| Expected | `DomainError::Expected` | End user (via i18n) | `trace_debug` | No — message goes to user |
| Unrecoverable | `DomainError::Unrecoverable` | Developer (logs only) | `trace_error` | Yes — `[Struct::method]` prefix |

This binary split means every error decision reduces to one question:
**"Should the end user see this message?"**
- Yes → `Expected` with `trl()` i18n key, `trace_debug`.
- No  → `Unrecoverable` with `[Struct::method]` prefix, `trace_error`.

### Trace responsibility: one source, not every propagation

Each error is traced **exactly once**: at the site where the error value is
first produced.  Propagation through `?` operators never adds a second trace.

For Diesel errors specifically, the trace lives inside the `From` impl
(`infrastructure/query.rs`).  The `?` at every Diesel call site triggers
that `From` conversion, which emits `tracing::error!`.  Adding `.trace_error()`
before the `?` on the bare Diesel result would:

1. Produce a **duplicate** log entry — the `From` impl already emits one.
2. Trace the raw Diesel error text, losing the `DomainError::Unrecoverable`
   wrapper and its `[Struct::method]` prefix.
3. Add **runtime overhead** on every Diesel call (error or not) —
   `.trace_error()` always evaluates `tracing::error!` even when the error
   level is filtered out.

For **directly constructed** `DomainError` values (both `Expected` via
`.ok_or()` and `Unrecoverable` via `.map_err()`), the trace call is
mandatory — there is no intermediary conversion to delegate to.

---

## Error Type Hierarchy

```
StdResult<T, E>                    ← util::rename (alias for std::result::Result)
  ├── DomainResult<T>              ← DomainError (domain layer)
  │     ├── Expected   { variant: ExpectedVariant, message: String }
  │     └── Unrecoverable { message: String }
  └── UseCaseResult<T>             ← UseCaseError(DomainError) — transparent newtype
```

### DomainError (`src/domain.rs`)

```rust
pub enum ExpectedVariant {
    /// Validation / resource errors
    Argument,
    /// Authentication errors
    Authentication,
}

pub enum DomainError {
    Expected { variant: ExpectedVariant, message: String },
    Unrecoverable { message: String },
}
```

Convenience constructors **must** be used instead of struct literals:

```rust
impl DomainError {
    pub fn expected_argument(msg: String) -> Self { ... }
    pub fn expected_authentication(msg: String) -> Self { ... }
    pub fn unrecoverable(msg: String) -> Self { ... }
}
```

### UseCaseError (`src/usecase.rs`)

```rust
pub struct UseCaseError(DomainError);

impl From<DomainError> for UseCaseError { ... }   // upward conversion
impl AsRef<DomainError> for UseCaseError { ... }  // downward inspection
```

`UseCaseError` adds **no new variants**. It exists solely for layer isolation:
the API/HTTP layer matches on `UseCaseError` without importing `DomainError`.

---

## ErrorTrace Mechanism (`util::err`)

```rust
pub trait ErrorTrace {
    fn trace_debug(self) -> Self;   // Expected errors
    fn trace_info(self) -> Self;    // Info-level events
    fn trace_error(self) -> Self;   // Unrecoverable errors
}

impl<T, E> ErrorTrace for StdResult<T, E>
where E: Debug + Display { ... }
```

Implemented on `Result<T, E>`. When the result is `Err`, emits a tracing event
at the corresponding level containing the error's `Display` output.

### Trace Level Selection

| Error kind | Method | Rationale |
|------------|--------|-----------|
| `Expected` | `.trace_debug()` | Business errors are normal — don't pollute error logs |
| `Unrecoverable` | `.trace_error()` | Internal failures are serious — must surface in error dashboards |

### Where to call trace

**Only at the construction site.** When an error propagates through layers via
`?`, do **NOT** add another trace call in the intermediate function. Trace once,
where the error is born.

**Do:**
```rust
// Construction site — trace here
return Err(DomainError::expected_argument(trl("error-user-not-found")))
    .trace_debug();
```

**Do NOT:**
```rust
// Propagation site — do NOT trace again
let user = query.get_by_id(id).await?;  // Error already traced upstream
```

---

## Constructing Expected Errors

### Rules

1. Use `trl("error-xxx")` for the message — **never** hardcode a language string
2. No `[Struct::method]` prefix — the message goes directly to the user
3. Call `.trace_debug()` immediately after construction

### Pattern

```rust
use crate::util::err::ErrorTrace as _;
use crate::util::i18n::trl;

// When Diesel .optional()? returns None:
.ok_or(DomainError::expected_argument(trl("error-user-not-found")))
.trace_debug()?;

// For business validation failures:
if !invitation.verify_code(&params.invitation_code) {
    return Err(DomainError::expected_argument(trl("error-invalid-invitation-code")))
        .trace_debug();
}

// For update with zero affected rows:
if rows_affected == 0 {
    return Err(DomainError::expected_argument(trl("error-invitation-not-found")))
        .trace_debug();
}
```

### i18n Key Naming Convention

Keys live in `locales/{lang}/main.ftl`. Use `kebab-case`, prefixed with
`error-` for error messages. Keep the key descriptive of **what happened**,
not where it happened.

| Key | zh-CN | en-US |
|-----|-------|-------|
| `error-user-not-found` | 该用户不存在 | User not found |
| `error-no-pending-invitation` | 不存在待处理的邀请 | No pending invitation |
| `error-invitation-not-found` | 邀请记录不存在 | Invitation record not found |
| `error-invalid-invitation-code` | 无效的邀请码 | Invalid invitation code |

When adding a new Expected error:
1. Add the key to both `locales/zh-CN/main.ftl` and `locales/en-US/main.ftl`
2. Use `trl("error-new-key")` at the construction site

---

## Constructing Unrecoverable Errors

### Rules

1. **Always include `[StructName::method_name]` prefix** in the message
2. Append the technical detail (e.g., the inner error's `Display`)
3. Call `.trace_error()` immediately after construction

### Pattern

```rust
use crate::util::err::ErrorTrace as _;

// Pool connection failure:
.map_err(|e| DomainError::unrecoverable(format!(
    "[Query::run_in_transaction] error getting connection: {}", e
)))
.trace_error()?;

// External service failure:
.map_err(|e| DomainError::unrecoverable(format!(
    "[R2OssClient::put_signed] failed to generate presigned put URL: {}", e
)))
.trace_error()?;
```

### Diesel Error Auto-Conversion

Diesel errors (except `NotFound`) are converted to `Unrecoverable` via a
blanket `From` impl in `infrastructure/query.rs`.  The trace is performed
inside the `From` impl, so call sites use plain `?` without `.trace_error()`.

`NotFound` is excluded by always calling `.optional()?` before `.ok_or(...)`,

```rust
let info: UserInfo = t_user
    .filter(f_id.eq(id))
    .select(UserInfo::as_select())
    .first(conn)
    .await
    .optional()?          // NotFound → Ok(None), other errors → Err(Unrecoverable)
    .ok_or(DomainError::expected_argument(trl("error-user-not-found")))
    .trace_debug()?;      // None → Expected with i18n
```

---

## `#[tracing::instrument]` Placement Rules

See also: `tracing-usage-spec` skill for rationale on prohibited placements.

| Layer / Function kind | `#[tracing::instrument]`? | Rationale |
|-----------------------|---------------------------|-----------|
| Constructors (`new`, `from`, `generate_id`) | ❌ Never | Pure assignment, high frequency, no I/O |
| Domain model (`src/domain/model/`) | ❌ Never | Pure business logic, no I/O |
| Domain actor (`src/domain/actor/`) | ❌ Never | Pure business logic, no I/O |
| UseCase functions (`src/usecase/`) | ✅ Always | Orchestration boundary, full request lifecycle |
| Infra query | ✅ Permitted | Database I/O — useful for tracing individual query duration and parameters |
| Infra query delegate (QueryTransactional) | ❌ Never | Delegate wraps the call — the free function already has instrument |
| Infra external with `trace_*` | ✅ Always | External service calls — isolate latency |
| API handlers (`src/api/`) | ✅ Always | HTTP request entry boundary |
| Harness delegation (`src/api/harness.rs`) | ❌ Never | Pure delegation — underlying impls already instrumented. See `harness-spec` |
| Pure logic without `trace_*` | ❌ Never | No diagnostic benefit |

**Heuristic**: if a function is an orchestration boundary (usecase, API handler)
or an external service call, it benefits from `#[tracing::instrument]`. For
infra query free functions, `#[instrument]` is **permitted but not required**;
add it when the observability benefit outweighs the span overhead.

---

## Constructor Error Handling

Constructors fall into two categories with different rules.

### Domain Constructors (aggregate `new`, `generate_id`, `From`)

- No `#[tracing::instrument]`
- No `trace_*` calls
- These cannot fail — if they do, it's a programmer error that panics

### Infrastructure Constructors (`Query::new`, `R2ImagePool::from_env`, `JwtCodec::from_env`)

- No `#[tracing::instrument]`
- No `trace_*` calls
- Return `anyhow::Result<Self>` — errors are flat construction failures
- **Always use `.with_context()` (never `.context()`)** — the closure form saves a
  heap allocation for the error message string on the happy path.
- **Every `.with_context()` message must carry `[Struct::method]` prefix**:
  ```rust
  .with_context(|| "[R2ImagePool::from_env] R2_ACCOUNT_ID is not set")?
  .with_context(|| "[R2ImagePool::from_env] R2_BUCKET_NAME is not set")?
  ```
- These will **panic immediately in `main()`**, so tracing is unnecessary —
  the process hasn't started serving yet
- Still log a debug message on success with `[Struct::method]` prefix:
  ```rust
  tracing::debug!(
      bucket = %bucket,
      domain = %domain,
      "[R2ImagePool::from_env] configured",
  );
  ```

### API / Server Constructor (`server::serve`)

- Returns `anyhow::Result<()>` — server bootstrap errors are fatal
- **Always use `.with_context()` (never `.context()`)**:
  ```rust
  let listener = TcpListener::bind(&addr)
      .await
      .with_context(|| format!("[server::serve] failed to bind listener on {addr:?}"))?;

  axum::serve(listener, app)
      .await
      .with_context(|| "[server::serve] server error")
  ```

---

## Layer Summary

| Layer | Error type | Creates errors? | Trace at | Instrument? |
|-------|-----------|-----------------|----------|-------------|
| **Domain** | `DomainError` | Yes — via constructors + `trl()` / format with prefix | Construction site | No |
| **UseCase** | `UseCaseError` (wraps `DomainError`) | Rarely — mostly propagates via `?` | Only if creating a new error | Yes |
| **Infrastructure query** | `DomainError` | Yes — NotFound → Expected, pool fail → Unrecoverable | Construction site | Yes (free functions + Query impl) |
| **Infrastructure external** | `DomainError` | Yes — all external failures → Unrecoverable | Construction site | Yes |
| **API** | TBD | Converts `UseCaseError` → HTTP responses | TBD | Yes |

---

## Quick Reference: Error Construction Cheat Sheet

```rust
// ── Expected (user-facing) ─────────────────────────────────────────────────
// Pattern: DomainError::expected_{variant}(trl("error-xxx")).trace_debug()

// Not found:
.ok_or(DomainError::expected_argument(trl("error-user-not-found"))).trace_debug()?;

// Validation failure:
return Err(DomainError::expected_argument(trl("error-invalid-code"))).trace_debug();

// Authentication failure:
return Err(DomainError::expected_authentication(trl("error-unauthorized"))).trace_debug();

// ── Unrecoverable (internal) ───────────────────────────────────────────────
// Pattern: DomainError::unrecoverable(format!("[Struct::method] detail: {e}")).trace_error()

// Connection pool:
.map_err(|e| DomainError::unrecoverable(format!(
    "[Query::get_by_id] error getting connection: {}", e
))).trace_error()?;

// External API:
.map_err(|e| DomainError::unrecoverable(format!(
    "[JwtCodec::sign] error when encoding: {}", e
))).trace_error()?;

// Diesel errors (automatic via From impl — no explicit trace needed):
let row = diesel_query.execute(conn).await?;  // ? triggers From<diesel::Error>
```

---

## Anti-Patterns

**Do NOT:**

```rust
// ❌ Struct literal instead of constructor
DomainError::Unrecoverable { message: "something broke".into() }

// ❌ Expected with hardcoded language
DomainError::expected_argument("该用户不存在".into())

// ❌ Expected with [Struct::method] prefix (leaked to user)
DomainError::expected_argument("[UserQuery::get_by_id] 该用户不存在".into())

// ❌ Unrecoverable without [Struct::method] prefix (can't trace origin)
DomainError::unrecoverable(format!("error getting connection: {}", e))

// ❌ Double-tracing — trace at both construction and propagation
let err = DomainError::expected_argument(trl("x")).trace_debug();
return Err(err).trace_debug();  // second trace is redundant

// ❌ .context() on anyhow::Result — use .with_context() always
let val = std::env::var("KEY").context("[Struct::method] KEY is not set")?;

// ❌ Instrument on constructor
use tracing::instrument;
#[instrument]
pub fn new(qid: String, ...) -> Self { ... }

// ❌ Instrument on pure domain logic
use tracing::instrument;
#[instrument]
pub fn verify_password(&self, password: &str) -> bool { ... }
```

