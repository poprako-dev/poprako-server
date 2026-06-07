---
name: trait-def-spec
description: Trait documentation conventions: every trait and method needs /// doc comment, blank line between methods, prefer intra-doc [links].
---

# Trait Definition Specification

Every trait in poprako-r — public or private — must carry a `///` doc comment
that explains its role. Every method inside a trait must also carry a `///` doc
comment. Multi-method traits must have a blank line between consecutive method
signatures.

---

## 1. Trait-level doc comment (`///`)

**Always required.** The comment describes the **contract** the trait
represents, not the implementation.

```rust
/// Read-only persistence contract for [`UserAggr`].
///
/// Each method takes an immutable `&self` reference, suitable for
/// non-transactional queries backed by a connection pool.
#[async_trait]
pub trait UserQuery { ... }
```

Private marker traits also get `///`:

```rust
/// Blanket-impl marker: every [`QueryTransactional`] is a
/// [`MemberQueryMut`](crate::domain::query::member::MemberQueryMut).
trait MemberQuery: domain_query::member::MemberQueryMut {}
```

---

## 2. Method-level doc comment (`///`)

Every trait method must have a leading `///` line. Describe what the method
**does** and, when relevant, its error semantics.

```rust
pub trait UserQuery {
    /// Returns the user with the given ID, or an expected error if not found.
    async fn get_by_id(&self, id: &str) -> DomainResult<UserAggr>;
}
```

For methods with complex signatures, the doc comment goes above the `async fn`
line, **before** any attributes:

```rust
pub trait MemberInvitationQueryMut {
    /// Returns the most recent pending invitation for the given invitee
    /// qualified ID, or an expected error if none exists.
    async fn get_pending_by_invitee_qid(
        &mut self,
        invitee_qid: &str,
    ) -> DomainResult<MemberInvitation>;
}
```

---

## 3. Blank line between methods

When a trait has **two or more methods**, each method block must be separated
by exactly one blank line.

**Do:**

```rust
pub trait UserQuery {
    /// Returns the user with the given ID, or an expected error if not found.
    async fn get_by_id(&self, id: &str) -> DomainResult<UserAggr>;

    /// Returns credentials (hashed password) for the given qualified ID.
    async fn get_credentials_by_qid(&self, qid: &str) -> DomainResult<UserCredential>;

    /// Creates a new user from the registration form and returns the persisted
    /// aggregate.
    async fn create(&self, form: UserForm) -> DomainResult<UserAggr>;
}
```

**Do NOT:**

```rust
// ❌ No blank line between methods.
pub trait UserQuery {
    async fn get_by_id(&self, id: &str) -> DomainResult<UserAggr>;
    async fn get_credentials_by_qid(&self, qid: &str) -> DomainResult<UserCredential>;
}
```

The blank line goes **after** `;` (single-line signatures) or **after** the
closing `)` and `;` (multi-line signatures), before the next `///`.

A trait with **one method only** does not need internal blank lines.

---

## 4. Intra-doc links

Prefer Rust intra-doc links (`[` `]` syntax) over free-text type names when
referring to types, methods, or modules. This gives IDE navigation and
compile-time link checking.

| Link target | Syntax |
|---|---|
| Type in same crate | `` [`UserAggr`] `` |
| Trait in same crate | `` [`QueryTransactional`] `` |
| Method with path | `` [`run_in_transaction`](crate::domain::query::Transactional::run_in_transaction) `` |
| Type from dependency | `` [`OffsetDateTime`](time::OffsetDateTime) `` |

---

## 5. Layer-specific patterns

### Domain query traits (`src/domain/query/`)

- Describe whether the trait is **read-only** (`&self`) or **mutable**
  (`&mut self`), and the transaction context it belongs to.
- For mutable traits: mention `` **only** inside `Transactional::run_in_transaction` ``.

```rust
use async_trait::async_trait;

/// Mutable persistence contract for [`Member`](crate::domain::model::aggregate::member::Member),
/// used **only** inside a transaction via [`QueryTransactional`](crate::domain::query::QueryTransactional).
#[async_trait]
pub trait MemberQueryTransactional {
    /// Inserts a new member row from the creation form.
    async fn create(&mut self, form: MemberForm) -> DomainResult<Member>;
}
```

### Domain external traits (`src/domain/external/`)

- Name the external system or protocol (OSS, JWT).

### Domain model traits (`src/domain/model/`)

- `EventSink` / `EventEmit`: explain the event lifecycle (push during
  operation → pull after commit).
- `RoleViewable` / `RoleAssignable`: document the read/write split for role masks.

### Utility traits (`src/util.rs`)

- Explain the conversion or behavior.
- For `ErrorTrace`: include the level-selection table.

### Infrastructure marker traits (`src/infrastructure/query/`)

- Private, blanket-impl markers. Comment format:

```rust
/// Blanket-impl marker: every [`QueryTransactional`] is a
/// [`UserQueryMut`](crate::domain::query::user::UserQueryMut).
trait UserQueryMut: domain_query::user::UserQueryMut {}
```

---

## 6. Quick checklist

Before opening a PR with a new or modified trait, verify:

- [ ] Trait has a `///` doc comment.
- [ ] Every method has a `///` doc comment.
- [ ] Methods in multi-method traits are separated by one blank line.
- [ ] Intra-doc links use `[` `]` syntax where possible.
- [ ] `cargo doc` produces no broken-intra-link warnings for the crate.
