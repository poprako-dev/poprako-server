---
name: poprako-aggr-conventions
description: |
  Conventions for the domain aggregate layer in poprako-r.
  Use whenever writing or modifying code under src/domain/model/aggregate/.
---

# Poprako-r Aggregate Conventions

## Two kinds of aggregates

All domain aggregates fall into exactly one of two categories.

### 1. Read-model aggregates (query outputs)

Returned from the query layer. All fields are `pub` for ergonomic reading.
No constructor — created by `From<EntityRow>` impls in `infrastructure/query/entity/`.

```rust
// domain/model/aggregate/user.rs
pub struct UserAggr {
    pub id: String,
    pub qid: String,
    pub nickname: String,
    // ...
}
```

**Do NOT** add `new()` constructors. Construction happens exclusively through
`From<EntityRow> for Aggregate` conversions in the infrastructure entity layer.

**OK to have** domain methods on read aggregates:
```rust
impl User {
    pub fn generate_one_token(&self) -> UserToken { ... }
}
impl MemberInvitation {
    pub fn verify_code(&self, code: &str) -> bool { ... }
}
```

### 2. Input aggregates (command payloads / forms)

Used as input parameters to query trait methods (`create`, `update`, etc.).
**Must** provide a `pub fn new(...)` constructor. This constructor is the **only**
allowed way to build them from outside the defining module.

```rust
// domain/model/aggregate/user.rs
impl UserAggr {
    pub fn generate_id() -> String {
        format!("user-{}", uuid::Uuid::now_v7())
    }
}

pub struct UserForm {
    pub id: String,
    pub qid: String,
    pub nickname: String,
    pub password_hash: String,

    events: Vec<DomainEvent>,
}

impl UserForm {
    pub fn new(qid: String, nickname: String, password_hash: String) -> Self {
        Self {
            id: User::generate_id(),
            qid,
            nickname,
            password_hash,
            events: Vec::new(),
        }
    }
}
```

**Do NOT** construct input aggregates with struct literals outside their defining
module. Public fields are for **reading** only.

The `new()` constructor is also responsible for **ID generation**. Each aggregate
generates its own ID internally — no external `gen_id` helpers.

**Do NOT:**
```rust
// BAD — struct literal bypasses constructor
let form = UserForm {
    id: "x".into(),
    qid: "y".into(),
    nickname: "z".into(),
    password_hash: "pw".into(),
    events: Vec::new(),
};
```

**Do:**
```rust
// GOOD — constructor generates ID internally
let form = UserForm::new(qid, nickname, password_hash);
```

## Event-carrying aggregates

Input aggregates that produce domain events must:
1. Keep the `events` field **private**
2. Implement `EventSink` (to push events in)
3. Implement `EventEmit` (to pull events out after transaction commit)

```rust
impl EventSink for UserForm {
    fn push_event(&mut self, event: DomainEvent) {
        self.events.push(event);
    }
}

impl EventEmit for UserForm {
    fn pull_events(&mut self) -> Vec<DomainEvent> {
        std::mem::take(&mut self.events)
    }
}
```

The usecase layer pushes events into the form via `EventSink`, then the form
is passed to the query layer's `create`. After the transaction, events are
pulled and published.

## File organization

- One file per aggregate family under `domain/model/aggregate/`
- Co-locate related types: `UserAggr`, `UserToken`, `UserCredential`, `UserForm`,
  `UserInfoUpdate` all live in `user.rs`
- `Form` suffix for creation inputs (`UserForm`, `MemberForm`)
- `Update` suffix for PUT update inputs (`UserInfoUpdate`)
- No suffix for read aggregates (`UserAggr`, `Member`, `MemberInvitation`)

## `From` conversions

`From<EntityRow> for Aggregate` conversions live in the **entity** module
(`infrastructure/query/entity/user.rs`), **not** in the aggregate module.
The domain layer must not know about Diesel entity types.
