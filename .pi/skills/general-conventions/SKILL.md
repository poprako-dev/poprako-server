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
