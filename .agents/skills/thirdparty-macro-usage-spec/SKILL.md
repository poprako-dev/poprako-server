---
name: thirdparty-macro-usage-spec
description: "Third-party macro rules: attribute/derive macros must use `use` import + bare name. tracing event macros must use fully qualified path at call site."
---

# Third-Party Macro Usage Specification

All third-party (non-std) **attribute** and **derive** macros **must** be
brought into scope via `use` imports and invoked with their bare
(unqualified) name.  Do not use crate-qualified paths at the call site
e.g., `#[tracing::instrument]`).

**Exception**: `tracing` call-like event macros (`tracing::error!`,
`tracing::warn!`, `tracing::info!`, `tracing::debug!`, `tracing::trace!`)
must **always** be invoked with a fully qualified `tracing::` path at the
call site.  They are never imported into scope via `use`.

## Why

For `use`-imported macros (attribute and derive): Using bare names via `use`
imports follows standard Rust conventions and keeps the attribute call site
concise.  The `use` block at the top of the file already documents every
third-party crate in use.

For `tracing` event macros: Bare `error!`, `debug!`, etc. shadows the
identifiers in the `log` crate, which many dependencies use.  Requiring
fully qualified `tracing::error!` avoids ambiguity and makes the origin
explicit at the call site without needing to scan import lists.

## Rule

| Macro kind | Rule | Example |
|------------|------|---------|
| Attribute macros (`#[...]`) | `use` import + bare name | `#[instrument]` |
| Derive macros (`#[derive(...)]`) | `use` import + bare name | `#[derive(Serialize)]` |
| `tracing` event macros (`!`) | Fully qualified at call site | `tracing::error!(...)` |

## Macro import table

| Crate | Macro | Import / call-site form | Kind |
|-------|-------|-------------------------|------|
| `tracing` | `instrument` | `use tracing::instrument;` → `#[instrument]` | attribute |
| `tracing` | `error!`, `warn!`, `info!`, `debug!`, `trace!` | Fully qualified only: `tracing::error!(...)` | call-like (event) |
| `async_trait` | `async_trait` | `use async_trait::async_trait;` → `#[async_trait]` | attribute |
| `serde` | `Serialize` | `use serde::Serialize;` → `#[derive(Serialize)]` | derive |
| `serde` | `Deserialize` | `use serde::Deserialize;` → `#[derive(Deserialize)]` | derive |
| `diesel` | `Queryable` | `use diesel::prelude::*;` → `#[derive(Queryable)]` | derive |
| `diesel` | `Selectable` | `use diesel::prelude::*;` → `#[derive(Selectable)]` | derive |
| `diesel` | `Insertable` | `use diesel::prelude::*;` → `#[derive(Insertable)]` | derive |
| `diesel` | `AsChangeset` | `use diesel::prelude::*;` → `#[derive(AsChangeset)]` | derive |
| `fluent_templates` | `static_loader` | `use fluent_templates::static_loader;` | proc macro |
| `unic_langid` | `langid` | `use unic_langid::langid;` | proc macro |

> When adding a new third-party dependency that exports macros, add a row to
> the import table above.

## Do / Don't

### tracing: attribute macro (`#[instrument]`)

**Do NOT:**
```rust
// No `use tracing::instrument;` import — wrong

#[tracing::instrument]
async fn foo() { ... }
```

**Do:**
```rust
use tracing::instrument;

#[instrument]
async fn foo() { ... }
```

### tracing: event macros (`error!`, `warn!`, `info!`, `debug!`, `trace!`)

**Do NOT:**
```rust
// Never import tracing event macros into scope
use tracing::{error, warn, info, debug};

error!(
    error = %e,
    "[Struct::method] something broke",
);
```

**Do:**
```rust
tracing::error!(
    error = %e,
    "[Struct::method] something broke",
);

tracing::debug!(
    constraint = %name,
    "[From<diesel::Error>] unique violation",
);
```

### async_trait

**Do NOT:**
```rust
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

1. For attribute and derive macros: add `use` imports at the top of the file.
2. Use **bare names** for those macros at every invocation site.
3. For `tracing` event macros: use **fully qualified paths** at every call site;
   never import them into scope.
4. Search for violations with:
   ```bash
   rg "#\[tracing::instrument\]|#\[async_trait::async_trait\]|serde::(Serialize|Deserialize)" -g '*.rs' src/
   rg "\btracing::(error|warn|info|debug|trace)!" -g '*.rs' src/   # these are correct
   rg "\b(error|warn|info|debug|trace)!" -g '*.rs' src/            # suspect — may be bare
   ```
5. Re-run `cargo check` to confirm the code compiles.
