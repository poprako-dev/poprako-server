---
name: general-conventions
description: |
  General coding conventions for poprako-r across all Rust source layers.
  Use whenever writing, modifying, or reviewing Rust code in this repository,
  especially for visibility, struct field exposure, source comments, and
  project-wide style that is not specific to one layer.
---

# Poprako-r General Conventions

These rules apply across the Rust codebase. Layer-specific rules live in
separate skills such as `query-domain-spec`, `query-infra-spec`,
`aggregate-definition-spec`, `harness-spec`, `trait-def-spec`, and
`error-handling-spec`.

## Source comments

All source comments must be written in English:

- Line comments: `// ...`
- Block comments: `/* ... */`
- Doc comments: `/// ...` and `//! ...`

Source comments must describe the Rust code itself. Do not mention reference
implementation paths, reference language function names, or migration notes in
comments that will live in `src/`.

## Import style

Prefer importing the concrete item that is used in the file. Do not keep a
module partially qualified at the call site unless that qualification carries
useful meaning or avoids an actual ambiguity.

Use a partially qualified module path only for explicit exceptions, such as:

- API/HTTP code calling usecase functions, where the final submodule and
  function names may be identical. Import `crate::usecase`, then call
  `usecase::user::sign_up_user(...)`.
- Schema DSL imports and framework preludes where the local convention already
  expects a module-shaped namespace, such as Diesel `dsl::*` and
  `diesel::prelude::*`.
- Macros or associated paths that another project skill explicitly requires to
  stay fully qualified.

Every ordinary type, trait, value function, error type, and result alias should
be brought into scope with a `use` item, then referenced by its leaf name. This
is especially important for `DomainError`, `DomainResult`, `UseCaseError`,
`UseCaseResult`, and `ExpectedVariant`; do not write long paths such as
`crate::domain::result::DomainError` in signatures or matches when a normal
import is possible.

Do not use wildcard imports such as `use super::*` or `use module::*` in
implementation code — the only exception is `use super::*` inside `#[cfg(test)]
mod tests` blocks (see `test-spec`). Framework/schema imports covered by the
explicit exceptions above remain allowed.

Curly braces in `use` statements must satisfy the `check-use-braces` skill:
braces are allowed only at the final leaf segment. `use a::b::{c, d};` is valid;
`use a::{b, c::d};` is not.

## Prefer constructor over struct literal

When a type provides a `new()` constructor (with or without builder methods),
use it instead of writing a struct literal directly.

Struct literal syntax is reserved for destructuring patterns (`let Foo { a, b } = val;`)
and for types that have no constructor.

**Do:**

```rust
let entry = UserEntry::new(&form.id, &form.nickname, now);
let changes = UserAspect::new(now).nickname(&input.nickname);
```

**Do NOT:**

```rust
let entry = UserEntry {
    f_id: &form.id,
    f_nickname: &form.nickname,
    f_created_at: now,
};
```

## Ownership before cloning

Prefer the cheapest ownership form that preserves correctness:

1. Move owned values when the current use is the value's final use.
2. Borrow by reference when the callee does not need ownership.
3. Clone only when multiple owned values are genuinely required, or when an
   interface boundary accepts only a borrowed value but an owned result must be
   retained elsewhere.

Do not write `.clone()` at a value's final use just to keep earlier code shape
unchanged. Reorder construction or introduce a short-lived local when that
allows the final use to move the value cleanly.

## Visibility: only private and `pub`

Visibility qualifiers other than `pub` are forbidden everywhere. This includes:

- `pub(crate)`
- `pub(super)`
- `pub(in path)`
- `pub(self)`

Every item is either fully private, with no visibility qualifier, or fully
`pub`. There is no intermediate visibility tier in this project.

```rust
type Cache = std::collections::HashMap<String, Vec<u8>>;

pub struct Query {
    pool: Pool<AsyncPgConnection>,
}
```

Do not use restricted visibility:

```rust
pub(crate) fn helper() {}
pub(super) struct Internal {}
```

## Trait method calls: UFCS required

Every trait method called on a harness or harness-like reference must use
UFCS (Universal Function Call Syntax) — `Trait::method(instance, args)` — never
dot syntax (`instance.method(args)`).

**Rationale**: Harnesses in this project use `ForwardRefs` to delegate trait
methods to inner implementations. When the binding is invisible at the call
site, dot syntax hides which trait a method belongs to, making review harder
and enabling accidental resolution to the wrong trait.

**Applies to:**

- The `harn` parameter in usecase functions.
- Any variable typed as a harness (`Harness`, `HarnessBase`, `TestHarness`,
  `FakeHarness`, etc.) in any source file — production code, test code,
  examples, and benchmarks.
- The `query` parameter inside `transaction_scoped` closures (which is
  a `MemoryMockQueryTransactional` or `RdbQueryTransactional`).

```rust
// ✅ Correct — UFCS everywhere
UserQuery::get_by_id(harn, id).await?;
ImagePut::put_signed(harn, &key).await?;
Transactional::transaction_scoped(harn, move |query| {
    async move {
        UserQueryTransactional::create(query, &form).await?;
        Ok(())
    }.boxed()
}).await?;
ImageGet::get_signed(&harn, "page-1.png").await.unwrap();

// ❌ Wrong — dot syntax
harn.get_by_id(id).await?;
harn.put_signed(&key).await?;
harn.transaction_scoped(move |query| { ... }).await?;
harn.get_signed("page-1.png").await.unwrap();
```

**Exception**: Inherent methods (defined in `impl Harness` / `impl TestHarness`
blocks) may use dot syntax. These are not trait methods, so UFCS does not
apply:

```rust
harn.seed_user(user, credential);   // ✅ inherent method
harn.snapshot();                      // ✅ inherent method
```

> For detailed usecase-layer examples, see `implement-fullchain-spec` §6.1.

## Field visibility

Types that carry behavior through `impl` blocks keep their fields private.
Only pure data containers expose fields as `pub`.

| Category | Public fields? | Example |
| --- | --- | --- |
| Diesel entity structs (`*Row`, `*Entry`, `*Aspect`) | Yes | `pub f_id: String` |
| Domain value objects | Yes | `pub id: String` |
| Domain aggregate business fields | Yes | `pub nickname: String` |
| Event queues and private marker fields | No | `events`, `_m` |
| Query handles | No | `pool`, `state` |
| Transactional query handles | No | `conn`, `state` |
| Harnesses, effect sinks, external service wrappers | No | dependency fields |

```rust
pub struct UserRow {
    pub f_id: String,
    pub f_nickname: String,
}

pub struct RdbQuery {
    pool: Pool<AsyncPgConnection>,
}
```

Do not expose fields from logic-carrying types:

```rust
pub struct RdbQuery {
    pub pool: Pool<AsyncPgConnection>,
}
```
