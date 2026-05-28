---
name: thirdparty-macro-usage-spec
description: |
  Enforces that all third-party macros in poprako-r use `use` imports with
  bare names (e.g., `use tracing::instrument;` then `#[instrument]`).
  Use whenever writing or reviewing Rust code that imports or invokes macros
  from tracing, serde, async_trait, diesel, or any other external crate.
  Also use when adding new third-party dependencies that export macros.
---

# Third-Party Macro Usage Specification

All third-party (non-std) macros **must** be brought into scope via `use`
imports and invoked with their bare (unqualified) name. Do not use
crate-qualified paths at the call site (e.g., `#[tracing::instrument]`).

## Why

Using bare names via `use` imports follows standard Rust conventions and
keeps the call site concise. The `use` block at the top of the file
already documents every third-party crate in use, so readers can see
the origin without needing qualified paths on every invocation.

Benefits:
- **Brevity**: `#[instrument]` vs `#[tracing::instrument]` — less visual noise.
- **Consistency**: Derive macros like `#[derive(Serialize)]` look the same
  as built-in derives like `#[derive(Debug, Clone)]`.
- **Standard practice**: This is how the Rust ecosystem is designed to work;
  `use` imports are the norm.

## Rule

**All third-party macros must be imported via `use` and invoked with bare
names at the call site.**

No crate-qualified macro paths (e.g., `#[tracing::instrument]`, `#[serde::Serialize]`).

## Macro import table

| Crate | Macro | Import form | Kind |
|-------|-------|-------------|------|
| `tracing` | `instrument` | `use tracing::instrument;` | attribute |
| `async_trait` | `async_trait` | `use async_trait::async_trait;` | attribute |
| `serde` | `Serialize` | `use serde::Serialize;` | derive |
| `serde` | `Deserialize` | `use serde::Deserialize;` | derive |
| `diesel` | `Queryable` | `use diesel::prelude::*;` (or specific path) | derive |
| `diesel` | `Selectable` | `use diesel::prelude::*;` (or specific path) | derive |
| `diesel` | `Insertable` | `use diesel::prelude::*;` (or specific path) | derive |
| `diesel` | `AsChangeset` | `use diesel::prelude::*;` (or specific path) | derive |
| `fluent_templates` | `static_loader` | `use fluent_templates::static_loader;` | proc macro |
| `unic_langid` | `langid` | `use unic_langid::langid;` | proc macro |

## No exceptions

Every third-party macro follows the same rule: `use` import + bare name.
There are no special exceptions. Diesel derive macros (`Queryable`,
`Selectable`, `Insertable`, `AsChangeset`) are imported via
`use diesel::prelude::*;` or a specific import path, and used bare just like
any other derive. The `#[diesel(table_name = ...)]` helper attribute is not
a macro — it is automatically recognized by the diesel derive macros and
does not need any import.

> When adding a new third-party dependency that exports macros, add a row to
> the import table above.

## Do / Don't

### tracing

**Do NOT:**
```rust
// No `use tracing::instrument;` import — wrong

#[tracing::instrument]
async fn foo() { ... }

#[tracing::instrument(skip(x), level = Level::DEBUG)]
async fn bar(x: &str) { ... }
```

**Do:**
```rust
use tracing::instrument;

#[instrument]
async fn foo() { ... }

#[instrument(skip(x), level = Level::DEBUG)]
async fn bar(x: &str) { ... }
```

### async_trait

**Do NOT:**
```rust
// No `use async_trait::async_trait;` — wrong

#[async_trait::async_trait]
pub trait MyTrait { ... }
```

**Do:**
```rust
use async_trait::async_trait;

#[async_trait]
pub trait MyTrait { ... }
```

### serde

**Do NOT:**
```rust
// No `use serde::{Deserialize, Serialize};` — wrong

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct MyStruct { ... }
```

**Do:**
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct MyStruct { ... }
```

### diesel

**Do NOT:**
```rust
// No `use diesel::...` — wrong

#[derive(Queryable, Selectable)]
#[diesel(table_name = schema::t_user)]
pub struct Row { ... }
```

**Do:**
```rust
use diesel::prelude::*;

#[derive(Queryable, Selectable)]
#[diesel(table_name = schema::t_user)]
pub struct Row { ... }
```

## When adding new code

1. Add `use` imports for each third-party macro at the top of the file.
2. Use **bare names** at every invocation site (`#[instrument]`, `#[async_trait]`,
   `#[derive(Serialize, Deserialize)]`, etc.).
3. Search for existing qualified third-party macro usage with:
   ```bash
   rg "#\[tracing::instrument\]|#\[async_trait::async_trait\]|serde::(Serialize|Deserialize)" -g '*.rs' src/
   ```
4. Replace any qualified invocations with bare names and add `use` imports.
5. Re-run `cargo check` to confirm the code compiles.
