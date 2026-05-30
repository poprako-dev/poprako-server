---
name: poprako-conventions
description: |
  Coding conventions for the poprako-r project, specifically the infra/query
  layer (Diesel). Use whenever writing or modifying code under src/infrastructure/query/.
---

# Poprako-r Infra Query Conventions

## Type annotation on `let`, never turbofish in chain

On long method chains, annotate the `let` binding rather than
sprinkling turbofish (`::<Type>`) inside the chain.

**Do:**
```rust
let row: UserRow = t_user
    .filter(f_id.eq(user_id))
    .select(UserRow::as_select())
    .first(conn)
    .await?;
```

**Do NOT:**
```rust
let row = t_user
    .filter(f_id.eq(user_id))
    .select(UserRow::as_select())
    .first::<UserRow>(conn)
    .await?;
```

## Trace every error at its first production site

Every `DomainError` that is **constructed directly** at a call site must
call the appropriate `trace_*` method immediately at that site, before it
is propagated.

| Error kind | Trace method |
|------------|-------------|
| `DomainError::Expected` (argument, authentication) | `.trace_debug()` |
| `DomainError::Unrecoverable` | `.trace_error()` |

Diesel driver errors become `DomainError::Unrecoverable` through the
`From<diesel::result::Error>` impl in `infrastructure/query.rs`, which
already emits a `tracing::error!` event.  These conversion sites do **not**
need an additional `.trace_error()` — doing so would produce a redundant
log entry and adds runtime overhead on the hot path.

**Do:**
```rust
// Expected error: trace_debug at the construction site
.ok_or(DomainError::expected_argument(trl("error-user-not-found")))
.trace_debug()?;

// Diesel error: From impl handles tracing, no extra trace_* needed
let info: UserInfo = t_user
    .select(UserInfo::as_select())
    .first(conn)
    .await?;
```

**Do NOT:**
```rust
// Redundant: Diesel error already traced by From impl
let info: UserInfo = t_user
    .select(UserInfo::as_select())
    .first(conn)
    .await
    .trace_error()?;
```
