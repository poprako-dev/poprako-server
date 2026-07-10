---
name: aggregate-definition-spec
description: Aggregate struct rules for poprako-server. Covers four categories (Aggr/Form/Update/Patch), ID generation, field order, events field, and file organization.
---

# Aggregate Definition Specification

This document defines the **precise structural rules** for every struct
under `src/domain/model/aggregate/`. For naming conventions and the high-level
category split, see `poprako-aggr-conventions`.

---

## 1. Four aggregate categories

| Category | Suffix | ID source |
|----------|--------|-----------|
| **Read-model** | `Aggr` | From entity row |
| **Input: Form** | `Form` | `*Aggr::generate_id()` |
| **Input: Update** | `Update` | Caller provides |
| **Input: Patch** | `Patch` | Caller provides |

All construction uses **struct literal** syntax (`S { .. }`). There are no
`new()` constructors on aggregate types, except for aggregates that carry an
`events` field (see §5).

---

## 2. Read-model aggregates (`*Aggr`)

Every aggregate file **must** contain exactly one `*Aggr` struct.

### Construction

Read-model aggregates are constructed via struct literal in `From<EntityRow>`
conversions:

```rust
impl From<UserRow> for UserAggr {
    fn from(v: UserRow) -> Self {
        UserAggr {
            id: v.f_id,
            nickname: v.f_nickname,
            qid: v.f_qid,
            // ...
        }
    }
}
```

Conversions live in `src/infrastructure/query/entity/`.

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
}
```

---

## 3. Input aggregates: Form

### Construction

Callers construct via struct literal, generating the `id` from the sibling
`*Aggr::generate_id()`:

```rust
let form = MemberForm {
    id: MemberAggr::generate_id(),
    user_id: user.id.clone(),
    user_nickname: user.nickname.clone(),
    team_id,
    roles,
};
```

Aggregates that carry an `events` field cannot use struct literal
construction — see §5.

- `id` is generated via sibling `*Aggr::generate_id()`.

---

## 4. Input aggregates: Update / Patch

### Construction

Callers construct via struct literal, providing the `id` explicitly:

```rust
let update = UserInfoUpdate {
    id: existing_id,
    qid,
    nickname,
};
```

- `id` is provided by the caller (e.g., URL path parameter).

---

## 5. Aggregates with an `events` field

Any aggregate (of any category — Aggr, Form, Update, Patch) that carries an
`events` field is an exception to the no-`new()` rule:

- The `events` field is **private** (`events: Vec<Event>` without `pub`).
- A `new(id, ...)` constructor is provided that initializes `events:
  Vec::new()`.
- Struct literal construction is unavailable to callers outside the defining
  module because the private `events` field cannot be set from outside. Callers
  inside the same module (tests) must also use `new()` for consistency.

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
```

---

## 6. Field declaration order

Within any aggregate struct:

1. `pub id: String,`
2. Business fields (all `pub`)
3. Event field if present (`events: Vec<Event>` — **private** for any aggregate
   that carries one)

All fields are `pub`. Struct literal fields in call sites follow the same order
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
- [ ] All fields are `pub`.
- [ ] No `new()` constructors except for aggregates that carry an `events` field.
- [ ] `Form` ID generated via `Aggr::generate_id()`; `Update` / `Patch` takes caller-provided `id`.
- [ ] Constructor field order matches declaration order.
- [ ] `From<EntityRow>` conversion uses struct literal.
- [ ] No `Cre` suffix — use `Form`.
