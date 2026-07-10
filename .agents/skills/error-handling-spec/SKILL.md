---
name: error-handling-spec
description: Error construction and propagation rules for all layers. Covers Expected vs Unrecoverable, trl() keys, i18n, instrument placement, and diesel NotFound handling.
---

# Poprako-server Error Handling Specification

## Core Philosophy

All errors in poprako-server fall into exactly one of two categories:

| Category | Variant | Audience | Logged by | Contains location? |
|----------|---------|----------|-----------|---------------------|
| Expected | `DomainError::Expected` | End user (via i18n) | `#[instrument(err, ...)]` on the returning boundary | No — message goes to user |
| Unrecoverable | `DomainError::Unrecoverable` | Developer (logs only) | `#[instrument(err, ...)]` on the returning boundary | Yes — `[Struct::method]` prefix |

This binary split means every error decision reduces to one question:
**"Should the end user see this message?"**
- Yes → `Expected` with `trl()` i18n key.
- No  → `Unrecoverable` with `[Struct::method]` prefix.

### Trace responsibility: instrumented boundaries, not error constructors

`DomainError` is a data type, not a logging hook. It has no `.trace()` method.
Returned errors are recorded by `#[instrument(err, ...)]` on functions that
perform I/O or orchestrate a usecase.

Only the leaf node in the error propagation tree may log an error directly.
This is the first site where the error occurs and is usually the smallest span
in the span tree. Parent functions that merely propagate the error with `?`
must not add another direct `tracing::error!` / `tracing::debug!` event.

For Diesel errors specifically, the `From` impl (`src/infra/query.rs`) only
classifies the raw Diesel error into `DomainError`. The caller's
`#[instrument(err, ...)]` span records the returned error.

Do not add manual `tracing::error!` / `tracing::debug!` calls when a function can
return the error through an instrumented `Result` boundary. Manual tracing is
reserved for swallowed errors, fire-and-forget handlers, and other cases where
no error is returned.

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

## Returned Error Tracing

Use `#[instrument(err, ...)]` on fallible functions in observable layers:
usecase orchestration, infra query free functions, infra query `Query` impls,
infra external calls, and API handlers.

**Do:**
```rust
use tracing::instrument;

#[instrument(err, skip(conn), level = Level::DEBUG)]
pub async fn get_by_id(conn: &mut AsyncPgConnection, id: &str) -> DomainResult<UserAggr> {
    // ...
}
```

**Do NOT:**
```rust
return Err(DomainError::expected_argument(trl("error-user-not-found")).trace());
```

---

## Constructing Expected Errors

### Rules

1. Use `trl("error-xxx")` for the message — **never** hardcode a language string
2. No `[Struct::method]` prefix — the message goes directly to the user
3. Return the constructed `DomainError`; the instrumented caller records it

### Pattern

```rust
use crate::util::i18n::trl;

// When Diesel .optional()? returns None:
.ok_or_else(|| DomainError::expected_argument(trl("error-user-not-found")))?;

// For business validation failures:
if !invitation.verify_code(&params.invitation_code) {
    return Err(DomainError::expected_argument(trl("error-invalid-invitation-code")));
}

// For update with zero affected rows:
if rows_affected == 0 {
    return Err(DomainError::expected_argument(trl("error-invitation-not-found")));
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
3. Return the constructed `DomainError`; the instrumented caller records it

### Pattern

```rust
// Pool connection failure:
.map_err(|e| DomainError::unrecoverable(format!(
    "[Query::run_in_transaction] error getting connection: {}", e
)))?;

// External service failure:
.map_err(|e| DomainError::unrecoverable(format!(
    "[R2OssClient::put_signed] failed to generate presigned put URL: {}", e
)))?;
```

### Diesel Error Auto-Conversion

Diesel errors (except `NotFound`) are converted to `Unrecoverable` via a
blanket `From` impl in `src/infra/query.rs`. Call sites use plain `?`; the
instrumented function returning the `DomainResult` records the error.

`NotFound` is excluded by always calling `.optional()?` before `.ok_or_else(...)`,

```rust
let info: UserInfo = t_user
    .filter(f_id.eq(id))
    .select(UserInfo::as_select())
    .first(conn)
    .await
    .optional()?          // NotFound → Ok(None), other errors → Err(Unrecoverable)
    .ok_or_else(|| DomainError::expected_argument(trl("error-user-not-found")))?;
```

---

## `#[instrument(err, ...)]` Placement Rules

See also: `tracing-usage-spec` skill for rationale on prohibited placements.

| Layer / Function kind | `#[instrument(err, ...)]`? | Rationale |
|-----------------------|---------------------------|-----------|
| Constructors (`new`, `from`, `generate_id`) | ❌ Never | Pure assignment, high frequency, no I/O |
| Domain model (`src/domain/model/`) | ❌ Never | Pure business logic, no I/O |
| Domain actor (`src/domain/actor/`) | ❌ Never | Pure business logic, no I/O |
| UseCase functions (`src/usecase/`) | ✅ Always | Orchestration boundary, full request lifecycle |
| Infra query | ✅ Permitted | Database I/O — useful for tracing individual query duration and parameters |
| Infra query delegate (RepoTransactional) | ❌ Never | Delegate wraps the call — the free function already has instrument |
| Infra external returning `DomainResult` | ✅ Always | External service calls — isolate latency |
| API handlers (`src/api/`) | ✅ Always | HTTP request entry boundary |
| Harness delegation (`src/api/harness.rs`) | ❌ Never | Pure delegation — underlying impls already instrumented. See `harness-spec` |
| Pure logic without I/O | ❌ Never | No diagnostic benefit |

**Heuristic**: if a function is an orchestration boundary (usecase, API handler)
or an external service call, it benefits from `#[instrument(err, ...)]`. For
infra query free functions, `#[instrument(err, ...)]` is **permitted but not
required**; add it when the observability benefit outweighs the span overhead.

---

## Constructor Error Handling

Constructors fall into two categories with different rules.

### Domain Constructors (aggregate `new`, `generate_id`, `From`)

- No `#[instrument]`
- These cannot fail — if they do, it's a programmer error that panics

### Infrastructure Constructors (`Query::new`, `R2ImagePool::from_env`, `JwtCodec::from_env`)

- No `#[instrument]`
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
      .with_context(|| format!("[server::serve] failed to bind listener on {:?}", addr))?;

  axum::serve(listener, app)
      .await
      .with_context(|| "[server::serve] server error")
  ```

---

## Layer Summary

| Layer | Error type | Creates errors? | Trace at | Instrument? |
|-------|-----------|-----------------|----------|-------------|
| **Domain** | `DomainError` | Yes — via constructors + `trl()` / format with prefix | Returned by instrumented caller | No |
| **UseCase** | `UseCaseError` (wraps `DomainError`) | Rarely — mostly propagates via `?` | Function return via `err` | Yes |
| **Infrastructure query** | `DomainError` | Yes — NotFound → Expected, pool fail → Unrecoverable | Function return via `err` | Yes (free functions + Query impl) |
| **Infrastructure external** | `DomainError` | Yes — all external failures → Unrecoverable | Function return via `err` | Yes |
| **API** | TBD | Converts `UseCaseError` → HTTP responses | Function return via `err` | Yes |

---

## Quick Reference: Error Construction Cheat Sheet

```rust
// ── Expected (user-facing) ─────────────────────────────────────────────────
// Pattern: DomainError::expected_{variant}(trl("error-xxx"))

// Not found:
.ok_or_else(|| DomainError::expected_argument(trl("error-user-not-found")))?;

// Validation failure:
return Err(DomainError::expected_argument(trl("error-invalid-code")));

// Authentication failure:
return Err(DomainError::expected_authentication(trl("error-unauthorized")));

// ── Unrecoverable (internal) ───────────────────────────────────────────────
// Pattern: DomainError::unrecoverable(format!("[Struct::method] detail: {}", e))

// Connection pool:
.map_err(|e| DomainError::unrecoverable(format!(
    "[Query::get_by_id] error getting connection: {}", e
)))?;

// External API:
.map_err(|e| DomainError::unrecoverable(format!(
    "[JwtCodec::sign] error when encoding: {}", e
)))?;

// Diesel errors (automatic via From impl — no manual tracing needed):
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

// ❌ Manual tracing at construction — use #[instrument(err, ...)] on the caller
tracing::error!("failed");
return Err(DomainError::expected_argument(trl("x")));

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
