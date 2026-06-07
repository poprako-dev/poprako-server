---
name: query-domain-spec
description: "Domain query trait conventions: one file per aggregate, Query vs QueryTransactional split, &self vs &mut self, reference params, QueryTransactional supertrait registration."
---

# Domain Query Trait Specification

Rules in this document are **layer-specific** — they apply to the domain
query trait files under `src/domain/query/` and to the central
`src/domain/query.rs` aggregation module.

General trait conventions (doc comments, blank lines, intra-doc links) are
defined in `trait-def-spec`. This document only adds the rules that are
unique to the domain query layer.

---

## 1. One file per aggregate

Each aggregate gets its own file under `src/domain/query/`, named with
snake_case matching the aggregate module:

```
src/domain/query/
├── user.rs
├── member.rs
├── member_invitation.rs
└── ...
```

Never put queries for two different aggregates in the same file.

---

## 2. Two independent traits per aggregate

Every aggregate file defines **exactly two** persistence contracts. They are
parallel, independent traits — neither inherits from the other.

| Trait | `self` kind | Impl target | Purpose |
|-------|------------|-------------|---------|
| `{Aggr}Query` | `&self` | `Query` (pool) | Operations outside transactions |
| `{Aggr}QueryTransactional` | `&mut self` | `TransactionalQuery<'c>` | Operations inside transactions |

`Query` 和 `TransactionalQuery` 的关系可以理解为：业务上需要事务保证的操作
放到 `*QueryTransactional`，不需要的留在 `*Query`。两者没有层级或继承关系，
各自保持最精简。

Naming detail:
- The read-only trait uses `Query` (not `Qeury`).

```rust
use async_trait::async_trait;

#[async_trait]
pub trait UserQuery {
    async fn get_by_id(&self, id: String) -> DomainResult<UserAggr>;
}

#[async_trait]
pub trait UserQueryTransactional {
    async fn create(&mut self, form: UserForm) -> DomainResult<UserAggr>;
}
```

---

## 3. Method placement: Query vs QueryTransactional

Add a method to the trait that best reflects its business context:

- **`{Aggr}Query`** — for operations that run outside a transaction.
  This includes reads, lookups, existence checks, and also **writes that
  do not require transaction guarantees** (e.g. fire-and-forget inserts
  via optimistic concurrency). Uses `&self` because the backing `Query`
  struct borrows from a connection pool.
- **`{Aggr}QueryTransactional`** — for operations that must run inside
  `Transactional::run_in_transaction`: inserts, updates, deletes, or reads
  that acquire row locks (`SELECT ... FOR UPDATE`). Uses `&mut self` to
  enforce single-writer semantics inside the transaction scope.

Do **not** add the same method to both traits. Put each method in the
single most appropriate place. If an operation never needs a transaction,
it belongs in `*Query`. If it always runs inside one, it belongs in
`*QueryTransactional`.

---

## 4. `&mut self` for transactional traits only

Only `*QueryTransactional` traits use `&mut self`. The `*Query` trait
always uses `&self`.

The `&mut self` on transactional traits communicates two things:
- The underlying pinned connection is being borrowed exclusively.
- The caller is expected to be inside `run_in_transaction`.

```rust
// good — transactional trait
pub trait MemberQueryTransactional {
    async fn create(&mut self, form: MemberForm) -> DomainResult<Member>;
}

// good — read-only trait
pub trait UserQuery {
    async fn get_by_id(&self, id: String) -> DomainResult<UserAggr>;
}
```

---

## 5. Prefer reference types over owned types

All `async_trait` method parameters should use **reference** types (`&str`,
`&UserForm`, etc.) rather than owned types.

References avoid unnecessary cloning at the call site. The caller already
has the data on the stack or in an `Arc`; taking a reference lets the
implementation borrow it without moving ownership into the trait object.

```rust
// good — reference types
async fn get_by_id(&self, id: &str) -> DomainResult<UserAggr>;
async fn create(&mut self, form: &UserForm) -> DomainResult<UserAggr>;

// bad — owned (unnecessary clone at call site)
async fn get_by_id(&self, id: String) -> DomainResult<UserAggr>;
async fn create(&mut self, form: UserForm) -> DomainResult<UserAggr>;
```

This rule applies to **domain query trait signatures only**. Free functions
in the infra layer (which take `&mut AsyncPgConnection`) are not constrained
by it.

---

## 6. Keep each trait minimal

Each trait contains **only** the methods that its callers actually need.
Resist the urge to pre-emptively add "just in case" methods.

`*Query` and `*QueryTransactional` are not CRUD interfaces — they are the
precise set of operations required by the use-case layer. When the use case
grows, add the new method then.

---

## 7. `QueryTransactional` supertrait — registration point

When a new aggregate gets a `*QueryTransactional` trait, two things must
happen in `src/domain/query.rs`:

**a) Add a `use` import:**
```rust
use crate::domain::query::new_aggregate::NewAggregateQueryTransactional;
```

**b) Add the trait as a supertrait bound on `QueryTransactional`:**
```rust
pub trait QueryTransactional:
    Send
    + UserQueryTransactional
    + MemberQueryTransactional
    + MemberInvitationQueryTransactional
    + NewAggregateQueryTransactional  // ← add here
{}
```

**c) Update the blanket impl accordingly:**
```rust
impl<T> QueryTransactional for T where
    T: Send
        + UserQueryTransactional
        + MemberQueryTransactional
        + MemberInvitationQueryTransactional
        + NewAggregateQueryTransactional  // ← add here
{}
```

Without this step, `run_in_transaction` closures cannot access the new
aggregate's transactional methods. The Rust compiler will reject the call
site because `QueryTransactional` does not imply the new trait.

---

## 8. `run_in_transaction` — usage pattern from use cases

The canonical call site pattern in use-case code uses a `Box::pin` closure:

```rust
query.run_in_transaction(async move |txn: &mut QueryTransactional<'_>| {
    Box::pin(async move {
        let user = txn.create(user_form).await?;
        let member = txn.create(member_form).await?;
        Ok((user, member))
    })
}).await?;
```

The closure takes `&mut QueryTransactional<'_>`, through which all
`*QueryTransactional` methods are available via the supertrait chain.
