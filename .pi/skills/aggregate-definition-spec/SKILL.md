---
name: aggregate-definition-spec
description: |
  Structural specification for domain aggregate struct definitions in poprako-r.
  Covers the four aggregate categories (read-model, Form, Update, Patch),
  the `_p: PrivateMarker` compile-time guard, `new()` constructor requirements,
  ID generation rules, and file organization. Use whenever defining or modifying
  structs under src/domain/model/aggregate/.
---

# Aggregate Definition Specification

This document defines the **precise structural rules** for every struct
under `src/domain/model/aggregate/`. For naming conventions and the high-level
category split, see `poprako-aggr-conventions`.

---

## 1. Four aggregate categories

| Category | Suffix | Constructor | ID source | `_p` |
|----------|--------|-------------|-----------|------|
| **Read-model** | `Aggr` | `new(all_fields)` | From entity row | ✅ |
| **Input: Form** | `Form` | `new(biz_params)` | `*Aggr::generate_id()` | ✅ |
| **Input: Update** | `Update` | `new(id, biz_params)` | Caller provides | ✅ |
| **Input: Patch** | `Patch` | `new(id, optional_fields)` | Caller provides | ✅ |

---

## 2. Universal: `_p` marker

**Every** struct includes as its last field:

```rust
/// Private marker to forbid struct literal construction outside this module.
_p: PrivateMarker,
```

The `PrivateMarker` type is defined once in `src/domain/model/aggregate.rs` and
imported as `use crate::domain::model::aggregate::PrivateMarker;` in each
aggregate file.

The comment is **always** `///` (doc comment), with the **exact** text:
"Private marker to forbid struct literal construction outside this module."

---

## 3. Read-model aggregates (`*Aggr`)

Every aggregate file **must** contain exactly one `*Aggr` struct.

### Constructor

```rust
pub fn new(
    id: String,
    // ... all fields in declaration order
) -> Self {
    Self {
        id,
        // ...
        _p: PrivateMarker,
    }
}
```

- Name: `new`.
- Visibility: `pub`.
- Parameters: one per field, **in the same order** as the struct declaration.
- Called from `From<EntityRow>` conversions in `src/infrastructure/query/entity/`.

### Example

```rust
pub struct TeamAggr {
    pub id: String,
    pub name: String,
    pub description: String,
    pub avatar_key: String,
    pub avatar_uploaded: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,

    /// Private marker to forbid struct literal construction outside this module.
    _p: PrivateMarker,
}

impl TeamAggr {
    pub fn new(
        id: String,
        name: String,
        description: String,
        avatar_key: String,
        avatar_uploaded: bool,
        created_at: OffsetDateTime,
        updated_at: OffsetDateTime,
    ) -> Self {
        Self {
            id,
            name,
            description,
            avatar_key,
            avatar_uploaded,
            created_at,
            updated_at,
            _p: PrivateMarker,
        }
    }
}
```

---

## 4. Input aggregates: Form

### Constructor

```rust
pub fn new(biz_param_1: String, biz_param_2: String, ...) -> Self {
    Self {
        id: Aggr::generate_id(),
        // business fields
        _p: PrivateMarker,
    }
}
```

- ID generated via sibling `*Aggr::generate_id()`.
- No `id` parameter in the constructor signature.

---

## 5. Input aggregates: Update / Patch

### Constructor

```rust
/// Creates a new `FooUpdate`.
///
/// `id` is the existing entity ID (provided by the caller, not generated).
pub fn new(id: String, biz_params...) -> Self {
    Self {
        id,
        // business fields
        _p: PrivateMarker,
    }
}
```

- `id` is the **first** parameter, provided by the caller.
- Constructor has a `///` doc comment explaining the ID semantics.

---

## 6. Field declaration order

Within any aggregate struct:

1. `pub id: String,`
2. Business fields (all `pub`)
3. Event field if event-carrying (`events: Vec<Event>,` — no visibility keyword)
4. `/// Private marker ...` + `_p: PrivateMarker,`

Within the constructor `Self { .. }` block, fields are listed in the same order
as the struct declaration.

---

## 7. ID format

IDs follow the pattern `{prefix}-{uuid_v7}`:

| Aggregate | Prefix | Example |
|-----------|--------|---------|
| User | `user-` | `user-018f...` |
| Member | `member-` | `member-018f...` |
| MemberInvitation | `member_invitation-` | `member_invitation-018f...` |
| Team | `team-` | (not yet generated in Rust) |
| SysMail | `sys_mail-` | `sys_mail-018f...` |

UUID generation uses `Uuid::now_v7()`.

---

## 8. File organization

- One file per aggregate family under `src/domain/model/aggregate/`
- Every file **must** contain a `*Aggr` read-model struct
- Co-locate all related types in the same file:
  - Read-model `*Aggr`
  - Input `*Form`
  - Input `*Update`
  - Input `*Patch`
  - Helper value types (`UserToken`, `UserCredential`, etc.)
- Imports go at the top of each file

---

## 9. Quick checklist

- [ ] Category correctly identified (Aggr / Form / Update / Patch).
- [ ] `_p: PrivateMarker` field present with exact `///` comment.
- [ ] `pub fn new(...)` exists on every struct.
- [ ] `Form::new()` generates ID; `Update::new()` / `Patch::new()` takes `id: String` as first param.
- [ ] Constructor field order matches declaration order.
- [ ] `PrivateMarker` imported from `crate::domain::model::aggregate`.
- [ ] `From<EntityRow>` conversion calls `new(...)`, never struct literal.
- [ ] No `Cre` suffix — use `Form`.
