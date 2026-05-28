---
name: thirdparty-macro-usage-spec
description: |
  Enforces that all third-party macros in poprako-r use fully qualified paths
  (e.g., `tracing::instrument` instead of bare `instrument`). Use whenever
  writing or reviewing Rust code that imports or invokes macros from tracing,
  serde, async_trait, or any other external crate. Also use when adding new
  third-party dependencies that export macros.
---

# Third-Party Macro Usage Specification

All third-party (non-std) macros **must** be invoked with their fully qualified
path. Do not rely on `use` imports to bring macros into scope.

## Why

Short names pollute the global macro namespace and obscure the origin of the macro.
A reader seeing `#[instrument]` has to scan imports to know it comes from `tracing`.
Using `#[tracing::instrument]` is self-documenting — the crate of origin is visible
at the use site.

The same reasoning applies to derive macros: `#[derive(serde::Serialize)]` tells
you immediately that serde is involved, without needing to check imports.

## Rule

**All third-party macros must use crate-qualified paths at the call site.**
No bare macro names imported via `use`.

## Included macros

| Crate | Macro | Qualified form | Kind |
|-------|-------|----------------|------|
| `tracing` | `instrument` | `#[tracing::instrument(...)]` | attribute |
| `async_trait` | `async_trait` | `#[async_trait::async_trait]` | attribute |
| `serde` | `Serialize` | `#[derive(serde::Serialize)]` | derive |
| `serde` | `Deserialize` | `#[derive(serde::Deserialize)]` | derive |

## Excluded macros

The following are **exempt** from this rule and may use bare names:

| Crate | Macro | Reason |
|-------|-------|--------|
| `diesel` | `Queryable`, `Selectable`, `Insertable`, `AsChangeset` | These derive macros work with the `#[diesel(...)]` helper attribute; full qualification would conflict with `diesel::insert_into` etc. |
| `fluent_templates` | `static_loader` | Inline DSL macro; qualified form would break the DSL block. |
| `unic_langid` | `langid` | One-liner macro; qualified form `unic_langid::langid!(...)` is allowed but not required for readability. |

> When adding a new third-party dependency that exports macros, follow the same
> reasoning: if full qualification breaks DSL syntax or conflicts with the crate
> path used for function calls, document the exception here.

## Do / Don't

### tracing

**Do NOT:**
```rust
use tracing::instrument;

#[instrument]
async fn foo() { ... }

#[instrument(skip(x), level = Level::DEBUG)]
async fn bar(x: &str) { ... }
```

**Do:**
```rust
// No `use tracing::instrument;` import

#[tracing::instrument]
async fn foo() { ... }

#[tracing::instrument(skip(x), level = Level::DEBUG)]
async fn bar(x: &str) { ... }
```

### async_trait

**Do NOT:**
```rust
use async_trait::async_trait;

#[async_trait]
pub trait MyTrait { ... }
```

**Do:**
```rust
// No `use async_trait::async_trait;`

#[async_trait::async_trait]
pub trait MyTrait { ... }
```

### serde

**Do NOT:**
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct MyStruct { ... }
```

**Do:**
```rust
// No `use serde::{Deserialize, Serialize};`

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct MyStruct { ... }
```

## When adding new code

1. Search for existing bare (unqualified) third-party macro usage with:
   ```bash
   rg "use (tracing|serde|async_trait|anyhow|thiserror)::" -g '*.rs' src/
   ```
2. Remove any `use` imports for macros from the detected crates.
3. Replace bare macro invocations with their fully qualified forms.
4. Re-run `cargo check` to confirm the code compiles.
