---
name: query-domain-spec
description: |
  Domain-layer query trait conventions for poprako-r. Covers
  `src/domain/query/` trait naming, method placement rules (Query vs
  QueryTransactional split), parameter ownership, trait file organization,
  and the `QueryTransactional` supertrait aggregation point. Use whenever
  defining or modifying trait files under src/domain/query/.
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
| `{Aggr}Qeury` | `&self` | `Query` (pool) | Read-only queries outside transactions |
| `{Aggr}QueryTransactional` | `&mut self` | `TransactionalQuery<'c>` | Reads and writes inside a transaction |

`Query` 和 `TransactionalQuery` 的关系可以理解为：业务上需要事务保证的操作
放到 `*QueryTransactional`，不需要的留在 `*Qeury`。两者没有层级或继承关系，
各自保持最精简。

Naming detail:
- The read-only trait uses the typo spelling `Qeury` (e followed by u) as a
  deliberate and consistent convention in this codebase. Do not "fix" it to
  `Query`.

```rust
use async_trait::async_trait;

#[async_trait]
pub trait UserQeury {
    async fn get_by_id(&self, id: String) -> DomainResult<UserAggr>;
}

#[async_trait]
pub trait UserQeuryTransactional {
    async fn create(&mut self, form: UserForm) -> DomainResult<UserAggr>;
}
```

---

## 3. Method placement: Query vs QueryTransactional

Add a method to the trait that best reflects its business context:

- **`{Aggr}Qeury`** — for operations that are safe to run outside a
  transaction: pure reads, lookups, existence checks. Uses `&self` because
  the backing `Query` struct borrows from a connection pool.
- **`{Aggr}QueryTransactional`** — for operations that must run inside
  `Transactional::run_in_transaction`: inserts, updates, deletes, or reads
  that acquire row locks (`SELECT ... FOR UPDATE`). Uses `&mut self` to
  enforce single-writer semantics inside the transaction scope.

Do **not** add the same method to both traits. Put each method in the
single most appropriate place. If an operation never needs a transaction,
it belongs in `*Qeury`. If it always runs inside one, it belongs in
`*QueryTransactional`.

---

## 4. `&mut self` for transactional traits only

Only `*QueryTransactional` traits use `&mut self`. The `*Qeury` trait
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
pub trait UserQeury {
    async fn get_by_id(&self, id: String) -> DomainResult<UserAggr>;
}
```

---

## 5. Parameters prefer owned types (ownership over borrowing)

All trait method parameters should use **owned** types. Avoid borrowing
parameters (`&str`, `&Form`) in `async_trait` method signatures.

Owning parameters avoids the lifetime complications that `async_trait`
introduces with references — the trait object is `Box<dyn Future>` behind
the scenes, and borrowed arguments force explicit lifetime annotations that
quickly become unwieldy.

The caller moves ownership in; the implementation can then freely pass data
across `.await` points without worrying about the borrow living long enough.

```rust
// good — owned
async fn get_by_id(&self, id: String) -> DomainResult<UserAggr>;
async fn create(&mut self, form: UserForm) -> DomainResult<UserAggr>;

// bad — borrowed (avoid in async_trait signatures)
async fn get_by_id(&self, id: &str) -> DomainResult<UserAggr>;
```

This rule applies to **domain query trait signatures only**. Free functions
in the infra layer (which take `&mut AsyncPgConnection`) are not constrained
by it.

---

## 6. Keep each trait minimal

Each trait contains **only** the methods that its callers actually need.
Resist the urge to pre-emptively add "just in case" methods.

`*Qeury` and `*QueryTransactional` are not CRUD interfaces — they are the
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
    + UserQeuryTransactional
    + MemberQueryTransactional
    + MemberInvitationQueryTransactional
    + NewAggregateQueryTransactional  // ← add here
{}
```

**c) Update the blanket impl accordingly:**
```rust
impl<T> QueryTransactional for T where
    T: Send
        + UserQeuryTransactional
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
