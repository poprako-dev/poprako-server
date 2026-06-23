---
name: format-output-spec
description: Forbids `{ident}` inline captures in format strings. All placeholders must be positional `{}`. tracing macros must use structured key=value fields.
---

# Format Output Specification

All formatting calls (`format!`, `write!`, `println!`, `panic!`) must use
**positional `{}` placeholders with the corresponding values in the
argument list**.  The inline identifier capture syntax (`{ident}` /
`{ident:?}`) is forbidden everywhere.

`tracing` event macros (`error!`, `warn!`, `info!`, `debug!`) must use
**structured fields** (`key = %value` or `key = ?value`) — never format-string
interpolation.

## Rule

### format! / write! / println! / panic!

Each value must appear in the argument list, separated from the format string
literal.

**Do:**
```rust
let msg = format!("[MyStruct::method] failed to process {}: {}", id, err);
let msg = format!("step {step_num}: {action}", step_num = 3, action = "retry");
```

**Do NOT:**
```rust
// ❌ Inline identifier capture — variable appears inside the string
let msg = format!("[MyStruct::method] cache miss for {entity_id}: {err}");

// ❌ Inline capture with debug formatting
println!("result: {res:?}");
```

### tracing event macros

Fields must be listed as structured `key = %value` (Display) or
`key = ?value` (Debug) key-value pairs.  The message string comes last and
must NOT contain interpolated values.

**Do:**
```rust
tracing::error!(
    error = %e,
    entity_id = %id,
    "[MyStruct::method] failed to retrieve entity",
);
```

**Do NOT:**
```rust
// ❌ Values embedded in format string
tracing::error!(
    "[MyStruct::method] failed to retrieve entity ({}, {})",
    id, e,
);

// ❌ Inline identifier capture inside tracing message
tracing::error!(
    e = %err,
    "[MyStruct::method] entity lookup failed for {id}: {err_msg}",
);
```

## Rationale

1. **Explicit data flow**: Values in the argument list make the source of
   every data point visible at a glance.  Inline captures hide data flow
   inside the string literal.
2. **Structured observability** (`tracing`): `key = %value` pairs become
   first-class searchable fields in tracing backends.  Inline-captured
   identifiers embedded in the message string do **not** become top-level
   structured fields.
3. **Consistency**: The whole codebase uses one style, removing guesswork
   about which form to choose.

## Checklist

Before opening a PR, verify:

- [ ] No `{ident}` bare-identifier captures inside `format!`, `write!`,
  `println!`, or `panic!` string literals.
- [ ] `tracing` event macros use structured fields (`key = %value` or
  `key = ?value`) with a plain message string.
- [ ] `format!` / `write!` etc. use `{}` placeholders with explicit
  argument-list entries, or named `{name}` with `name = value`.
