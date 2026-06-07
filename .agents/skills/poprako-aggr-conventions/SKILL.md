---
name: poprako-aggr-conventions
description: Domain aggregate conventions: four categories (Aggr/Form/Update/Patch), struct literal construction, no new() except for events-carrying aggregates, From conversions in entity module.
---

# Poprako-r Aggregate Conventions

## Universal rule

All structs under `src/domain/model/aggregate/` use **struct literal**
construction (`S { .. }`). There are no `new()` constructors on aggregate
types, **except** for aggregates that carry an `events` field (see
"Aggregates with an `events` field" below). All fields are `pub`, **except**
the `events` field on aggregates that carry one, which is private.

---

## Four categories of aggregates

| Category | Suffix | Generates ID? | Examples |
|----------|--------|--------------|----------|
| **Read-model** | `Aggr` | ❌ | `UserAggr`, `MemberAggr` |
| **Input: Form** | `Form` | ✅ (via `Aggr::generate_id()`) | `UserForm`, `MemberForm` |
| **Input: Update** | `Update` | ❌ (caller provides) | `UserInfoUpdate` |
| **Input: Patch** | `Patch` | ❌ (caller provides) | — (none yet) |

### 1. Read-model aggregates (`*Aggr`)

The primary persistent entity for each aggregate file. Every file **must** have
exactly one `*Aggr` struct as its read model.

Constructed via `From<EntityRow>` conversions in
`src/infrastructure/query/entity/`. The `From` implementation uses struct
literal syntax.

```rust
pub struct UserAggr {
    pub id: String,
    pub qid: String,
    pub nickname: String,
    // ...
}

impl UserAggr {
    pub fn generate_id() -> String {
        format!("user-{}", Uuid::now_v7())
    }

    pub fn generate_avatar_key(&self) -> String {
        format!("user_avatar/{}", self.id)
    }
}
```

### 2. Input aggregates — Form (`*Form`)

Creation payload. Callers generate the ID via the sibling `*Aggr`'s
`generate_id()` and construct via struct literal.

```rust
pub struct MemberForm {
    pub id: String,
    pub user_id: String,
    pub user_nickname: String,
    pub team_id: String,
    pub roles: RoleMask,
}

// Construction at call site:
let form = MemberForm {
    id: MemberAggr::generate_id(),
    user_id,
    user_nickname,
    team_id,
    roles,
};
```

Aggregates that carry an `events` field cannot use struct literal construction
— see "Aggregates with an `events` field" below.

### 3. Input aggregates — Update (`*Update`)

PUT update payload. The caller provides the `id` explicitly.

```rust
pub struct UserInfoUpdate {
    pub id: String,
    pub qid: String,
    pub nickname: String,
}

// Construction at call site:
let update = UserInfoUpdate {
    id: existing_id,
    qid,
    nickname,
};
```

### 4. Input aggregates — Patch (`*Patch`)

PATCH update payload. Like `Update`, the `id` is caller-provided. Differs from
`Update` in that only some fields are present (optional).

---

## Aggregates with an `events` field

Any aggregate (of any category — Aggr, Form, Update, Patch) that carries an
`events` field must:
1. Keep the `events` field **private** (`events: Vec<Event>` without `pub`).
2. Provide a `new(id, ...)` constructor that initializes `events: Vec::new()`.
3. Implement `EventSink` (to push events in).
4. Implement `EventEmit` (to pull events out after transaction commit).

The `events` field is placed last among the fields in the struct layout.

```rust
pub struct UserForm {
    pub id: String,
    pub qid: String,
    pub nickname: String,
    pub password_hash: String,

    events: Vec<Event>,  // private — must use new()
}

impl UserForm {
    pub fn new(id: String, qid: String, nickname: String, password_hash: String) -> Self {
        Self {
            id,
            qid,
            nickname,
            password_hash,
            events: Vec::new(),
        }
    }
}

// Construction at call site:
let mut form = UserForm::new(
    UserAggr::generate_id(),
    qid,
    nickname,
    password_hash,
);
```

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

The `From` impl uses struct literal syntax:

```rust
impl From<UserRow> for UserAggr {
    fn from(v: UserRow) -> Self {
        UserAggr {
            id: v.f_id,
            nickname: v.f_nickname,
            qid: v.f_qid,
            is_sadmin: v.f_is_sadmin,
            avatar_key: v.f_avatar_key.unwrap_or_default(),
            avatar_uploaded: v.f_avatar_uploaded,
            last_active_at: v.f_last_active_at,
            created_at: v.f_created_at,
            updated_at: v.f_updated_at,
        }
    }
}
```

---

## Quick checklist

- [ ] Every aggregate file has a `*Aggr` read-model struct.
- [ ] No `new()` constructors except for aggregates that carry an `events` field.
- [ ] All fields are `pub`, except `events` on aggregates that carry one is private.
- [ ] No `_m` / `PrivateMarker` fields.
- [ ] `Form` ID generated via `Aggr::generate_id()`; `Update` / `Patch` takes caller-provided `id`.
- [ ] `From<EntityRow>` conversions use struct literal, not `new(...)`.
- [ ] No `Cre` suffix — use `Form`.
