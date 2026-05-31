---
name: poprako-aggr-conventions
description: |
  Conventions for the domain aggregate layer in poprako-r.
  Use whenever writing or modifying code under src/domain/model/aggregate/.
---

# Poprako-r Aggregate Conventions

## Universal rule

**Every** struct defined under `src/domain/model/aggregate/` carries a private
marker field `_p: PrivateMarker` and provides a `pub fn new(...)` constructor.
Struct literal construction (`S { .. }`) is forbidden outside the defining
module — enforced at compile time by the private marker.

---

## Four categories of aggregates

| Category | Suffix | Constructor signature | Generates ID? | Examples |
|----------|--------|-----------------------|--------------|----------|
| **Read-model** | `Aggr` | `new(all fields)` | ❌ | `UserAggr`, `MemberAggr` |
| **Input: Form** | `Form` | `new(biz_params)` | ✅ | `UserForm`, `MemberForm` |
| **Input: Update** | `Update` | `new(id, biz_params)` | ❌ (caller provides) | `UserInfoUpdate` |
| **Input: Patch** | `Patch` | `new(id, optional_fields)` | ❌ (caller provides) | — (none yet) |

### 1. Read-model aggregates (`*Aggr`)

The primary persistent entity for each aggregate file. Every file **must** have
exactly one `*Aggr` struct as its read model.

Constructed via `From<EntityRow>` conversions in
`src/infrastructure/query/entity/`. The `From` implementation calls `new(...)`
with all fields.

```rust
pub struct UserAggr {
    pub id: String,
    pub qid: String,
    pub nickname: String,
    // ...

    /// Private marker to forbid struct literal construction outside this module.
    _p: PrivateMarker,
}

impl UserAggr {
    pub fn generate_id() -> String {
        format!("user-{}", Uuid::now_v7())
    }

    pub fn generate_avatar_key(&self) -> String {
        format!("user_avatar/{}", self.id)
    }

    pub fn new(
        id: String,
        qid: String,
        nickname: String,
        // ... all fields in declaration order
    ) -> Self {
        Self {
            id,
            qid,
            nickname,
            // ...
            _p: PrivateMarker,
        }
    }
}
```

### 2. Input aggregates — Form (`*Form`)

Creation payload. Constructor generates its own ID via the sibling `*Aggr`'s
`generate_id()`.

```rust
pub struct UserForm {
    pub id: String,
    pub qid: String,
    pub nickname: String,
    pub password_hash: String,

    events: Vec<Event>,

    /// Private marker to forbid struct literal construction outside this module.
    _p: PrivateMarker,
}

impl UserForm {
    pub fn new(qid: String, nickname: String, password: String) -> Self {
        Self {
            id: UserAggr::generate_id(),
            qid,
            nickname,
            password_hash: password,
            events: Vec::new(),
            _p: PrivateMarker,
        }
    }
}
```

### 3. Input aggregates — Update (`*Update`)

PUT update payload. Constructor receives the `id` from the caller (e.g., URL
path parameter).

```rust
pub struct UserInfoUpdate {
    pub id: String,
    pub qid: String,
    pub nickname: String,

    /// Private marker to forbid struct literal construction outside this module.
    _p: PrivateMarker,
}

impl UserInfoUpdate {
    /// Creates a new `UserInfoUpdate`.
    ///
    /// `id` is the existing user ID (provided by the caller, not generated).
    pub fn new(id: String, qid: String, nickname: String) -> Self {
        Self {
            id,
            qid,
            nickname,
            _p: PrivateMarker,
        }
    }
}
```

### 4. Input aggregates — Patch (`*Patch`)

PATCH update payload. Like `Update`, the `id` is caller-provided. Differs from
`Update` in that only some fields are present (optional).

---

## `PrivateMarker`

Defined once in `src/domain/model/aggregate.rs`:

```rust
/// Zero-sized marker type that prevents struct literal construction
/// of input aggregates from outside the defining module.
///
/// Include `_p: PrivateMarker` as a field in any aggregate struct
/// whose construction should be limited to `new()` constructors.
#[derive(Default, Clone, Copy)]
pub struct PrivateMarker;

impl std::fmt::Debug for PrivateMarker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}
```

Import in each aggregate file as:

```rust
use crate::domain::model::aggregate::PrivateMarker;
```

---

## Event-carrying aggregates

Input aggregates that produce domain events must:
1. Keep the `events` field **private**
2. Implement `EventSink` (to push events in)
3. Implement `EventEmit` (to pull events out after transaction commit)

The `events` field is placed **before** the `_p` marker in the struct layout.

---

## File organization

- One file per aggregate family under `src/domain/model/aggregate/`
- Every file **must** contain a `*Aggr` read-model struct as its primary type
- Co-locate related types: `UserAggr`, `UserToken`, `UserCredential`, `UserForm`,
  `UserInfoUpdate` all live in `user.rs`
- `Form` suffix for creation inputs (`UserForm`, `MemberForm`, `SysMailForm`)
- `Update` suffix for PUT update inputs (`UserInfoUpdate`)
- `Patch` suffix for PATCH update inputs
- `Aggr` suffix for read models (`UserAggr`, `MemberAggr`, `TeamAggr`, `SysMailAggr`)

---

## `From` conversions

`From<EntityRow> for Aggregate` conversions live in the **entity** module
(`src/infrastructure/query/entity/`), **not** in the aggregate module.
The domain layer must not know about Diesel entity types.

The `From` impl calls `new(...)`:

```rust
impl From<UserInfo> for UserAggr {
    fn from(v: UserInfo) -> Self {
        UserAggr::new(
            v.f_id,
            v.f_nickname,
            v.f_qid,
            v.f_is_sadmin,
            v.f_avatar_key.unwrap_or_default(),
            v.f_avatar_uploaded,
            v.f_last_active_at,
            v.f_created_at,
            v.f_updated_at,
        )
    }
}
```

---

## Quick checklist

- [ ] Every aggregate file has a `*Aggr` read-model struct.
- [ ] Every struct has `_p: PrivateMarker` with `///` doc comment.
- [ ] Every struct has a `pub fn new(...)` constructor.
- [ ] No struct literal construction of aggregate types outside their module.
- [ ] `Form::new()` generates ID; `Update::new()` / `Patch::new()` takes `id: String` as first param.
- [ ] `From<EntityRow>` conversions call `new(...)`, not struct literal.
- [ ] No `Cre` suffix — use `Form`.
