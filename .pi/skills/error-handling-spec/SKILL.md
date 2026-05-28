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
| Expected | `DomainErr::Expected` | End user (via i18n) | `trace_debug` | No — message goes to user |
| Unrecoverable | `DomainErr::Unrecoverable` | Developer (logs only) | `trace_error` | Yes — `[Struct::method]` prefix |

This binary split means every error decision reduces to one question:
**"Should the end user see this message?"**
- Yes → `Expected` with `trl()` i18n key, `trace_debug`.
- No  → `Unrecoverable` with `[Struct::method]` prefix, `trace_error`.

---

## Error Type Hierarchy

```
StdResl<T, E>                    ← util::rename (alias for std::result::Result)
  ├── DomainResl<T>              ← DomainErr (domain layer)
  │     ├── Expected   { variant: ExpectedErr, message: String }
  │     └── Unrecoverable { message: String }
  └── UseCaseResl<T>             ← UseCaseErr(DomainErr) — transparent newtype
```

### DomainErr (`src/domain.rs`)

```rust
pub enum ExpectedErr {
    /// Validation / resource errors
    Argument,
    /// Authentication errors
    Authentication,
}

pub enum DomainErr {
    Expected { variant: ExpectedErr, message: String },
    Unrecoverable { message: String },
}
```

Convenience constructors **must** be used instead of struct literals:

```rust
impl DomainErr {
    pub fn expected_argument(msg: String) -> Self { ... }
    pub fn expected_authentication(msg: String) -> Self { ... }
    pub fn unrecoverable(msg: String) -> Self { ... }
}
```

### UseCaseErr (`src/usecase.rs`)

```rust
pub struct UseCaseErr(DomainErr);

impl From<DomainErr> for UseCaseErr { ... }   // upward conversion
impl AsRef<DomainErr> for UseCaseErr { ... }  // downward inspection
```

`UseCaseErr` adds **no new variants**. It exists solely for layer isolation:
the API/HTTP layer matches on `UseCaseErr` without importing `DomainErr`.

---

## ErrorTrace Mechanism (`util::err`)

```rust
pub trait ErrorTrace {
    fn trace_debug(self) -> Self;   // Expected errors
    fn trace_info(self) -> Self;    // Info-level events
    fn trace_error(self) -> Self;   // Unrecoverable errors
}

impl<T, E> ErrorTrace for StdResl<T, E>
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
return Err(DomainErr::expected_argument(trl("error-user-not-found")))
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
.ok_or(DomainErr::expected_argument(trl("error-user-not-found")))
.trace_debug()?;

// For business validation failures:
if !invitation.verify_code(&params.invitation_code) {
    return Err(DomainErr::expected_argument(trl("error-invalid-invitation-code")))
        .trace_debug();
}

// For update with zero affected rows:
if rows_affected == 0 {
    return Err(DomainErr::expected_argument(trl("error-invitation-not-found")))
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
.map_err(|e| DomainErr::unrecoverable(format!(
    "[Query::run_in_transaction] error getting connection: {}", e
)))
.trace_error()?;

// External service failure:
.map_err(|e| DomainErr::unrecoverable(format!(
    "[R2OssClient::put_signed] failed to generate presigned put URL: {}", e
)))
.trace_error()?;
```

### Diesel Error Auto-Conversion

Diesel errors (except `NotFound`) are automatically converted to `Unrecoverable`
via a blanket `From` impl:

```rust
// infrastructure/query.rs
impl From<diesel::result::Error> for DomainErr {
    fn from(val: diesel::result::Error) -> Self {
        let err = DomainErr::unrecoverable(val.to_string());
        tracing::error!("[trace_error] {}", err);
        err
    }
}
```

Note: The trace is performed inside the `From` impl because call sites use `?`
without an explicit `.trace_error()`. The message uses Diesel's own `Display`,
which includes query context.

**NotFound is excluded** from this conversion by always calling `.optional()?`
before `.ok_or(...)`, which maps `NotFound` to `Ok(None)`:

```rust
let info: UserInfo = t_user
    .filter(f_id.eq(id))
    .select(UserInfo::as_select())
    .first(conn)
    .await
    .optional()?          // NotFound → Ok(None), other errors → Err(Unrecoverable)
    .ok_or(DomainErr::expected_argument(trl("error-user-not-found")))
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
| Infra query with `trace_*` | ✅ Always | Database I/O — observe duration and parameters |
| Infra query delegate (TransactionalQuery) | ❌ Never | Delegate wraps the call — the free function already has instrument |
| Infra external with `trace_*` | ✅ Always | External service calls — isolate latency |
| API handlers (`src/api/`) | ✅ Always | HTTP request entry boundary |
| Pure logic without `trace_*` | ❌ Never | No diagnostic benefit |

**Heuristic**: if a function calls `.trace_debug()` or `.trace_error()`, and
it is not a constructor or pure logic, it needs `#[tracing::instrument]`.

---

## Constructor Error Handling

Constructors fall into two categories with different rules.

### Domain Constructors (aggregate `new`, `generate_id`, `From`)

- No `#[tracing::instrument]`
- No `trace_*` calls
- These cannot fail — if they do, it's a programmer error that panics

### Infrastructure Constructors (`Query::new`, `R2OssClient::from_env`)

- No `#[tracing::instrument]`
- No `trace_*` calls
- Return `anyhow::Result<Self>` — errors are flat construction failures
- **Every `.context()` message must carry `[Struct::new]` prefix**:
  ```rust
  .context("[R2OssClient::new] R2_ACCOUNT_ID is not set")?
  .context("[R2OssClient::new] R2_BUCKET_NAME is not set")?
  ```
- These will **panic immediately in `main()`**, so tracing is unnecessary —
  the process hasn't started serving yet
- Still log a debug message on success with `[Struct::new]` prefix:
  ```rust
  tracing::debug!(
      bucket = %bucket,
      domain = %domain,
      "[R2OssClient::new] configured",
  );
  ```

---

## Layer Summary

| Layer | Error type | Creates errors? | Trace at | Instrument? |
|-------|-----------|-----------------|----------|-------------|
| **Domain** | `DomainErr` | Yes — via constructors + `trl()` / format with prefix | Construction site | No |
| **UseCase** | `UseCaseErr` (wraps `DomainErr`) | Rarely — mostly propagates via `?` | Only if creating a new error | Yes |
| **Infrastructure query** | `DomainErr` | Yes — NotFound → Expected, pool fail → Unrecoverable | Construction site | Yes (free functions + Query impl) |
| **Infrastructure external** | `DomainErr` | Yes — all external failures → Unrecoverable | Construction site | Yes |
| **API** | TBD | Converts `UseCaseErr` → HTTP responses | TBD | Yes |

---

## Quick Reference: Error Construction Cheat Sheet

```rust
// ── Expected (user-facing) ─────────────────────────────────────────────────
// Pattern: DomainErr::expected_{variant}(trl("error-xxx")).trace_debug()

// Not found:
.ok_or(DomainErr::expected_argument(trl("error-user-not-found"))).trace_debug()?;

// Validation failure:
return Err(DomainErr::expected_argument(trl("error-invalid-code"))).trace_debug();

// Authentication failure:
return Err(DomainErr::expected_authentication(trl("error-unauthorized"))).trace_debug();

// ── Unrecoverable (internal) ───────────────────────────────────────────────
// Pattern: DomainErr::unrecoverable(format!("[Struct::method] detail: {e}")).trace_error()

// Connection pool:
.map_err(|e| DomainErr::unrecoverable(format!(
    "[Query::get_by_id] error getting connection: {}", e
))).trace_error()?;

// External API:
.map_err(|e| DomainErr::unrecoverable(format!(
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
DomainErr::Unrecoverable { message: "something broke".into() }

// ❌ Expected with hardcoded language
DomainErr::expected_argument("该用户不存在".into())

// ❌ Expected with [Struct::method] prefix (leaked to user)
DomainErr::expected_argument("[UserQuery::get_by_id] 该用户不存在".into())

// ❌ Unrecoverable without [Struct::method] prefix (can't trace origin)
DomainErr::unrecoverable(format!("error getting connection: {}", e))

// ❌ Double-tracing — trace at both construction and propagation
let err = DomainErr::expected_argument(trl("x")).trace_debug();
return Err(err).trace_debug();  // second trace is redundant

// ❌ Instrument on constructor
#[tracing::instrument]
pub fn new(qid: String, ...) -> Self { ... }

// ❌ Instrument on pure domain logic
#[tracing::instrument]
pub fn verify_password(&self, password: &str) -> bool { ... }
```

