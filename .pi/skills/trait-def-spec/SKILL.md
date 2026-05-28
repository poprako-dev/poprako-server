---
name: trait-def-spec
description: |
  Enforces documentation and formatting conventions for all trait definitions
  in poprako-r, across all layers (domain, external, infra). Use whenever
  defining, modifying, or reviewing any `trait` block in the codebase.
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
#[async_trait::async_trait]
pub trait UserQeury { ... }
```

Private marker traits also get `///`:

```rust
/// Blanket-impl marker: every [`TransactionalQuery`] is a
/// [`MemberQueryMut`](crate::domain::query::member::MemberQueryMut).
trait MemberQuery: domain_query::member::MemberQueryMut {}
```

---

## 2. Method-level doc comment (`///`)

Every trait method must have a leading `///` line. Describe what the method
**does** and, when relevant, its error semantics.

```rust
pub trait UserQeury {
    /// Returns the user with the given ID, or an expected error if not found.
    async fn get_by_id(&self, id: &str) -> DomainResl<UserAggr>;
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
    ) -> DomainResl<MemberInvitation>;
}
```

---

## 3. Blank line between methods

When a trait has **two or more methods**, each method block must be separated
by exactly one blank line.

**Do:**

```rust
pub trait UserQeury {
    /// Returns the user with the given ID, or an expected error if not found.
    async fn get_by_id(&self, id: &str) -> DomainResl<UserAggr>;

    /// Returns credentials (hashed password) for the given qualified ID.
    async fn get_credentials_by_qid(&self, qid: &str) -> DomainResl<UserCredential>;

    /// Creates a new user from the registration form and returns the persisted
    /// aggregate.
    async fn create(&self, form: UserForm) -> DomainResl<UserAggr>;
}
```

**Do NOT:**

```rust
// ❌ No blank line between methods.
pub trait UserQeury {
    async fn get_by_id(&self, id: &str) -> DomainResl<UserAggr>;
    async fn get_credentials_by_qid(&self, qid: &str) -> DomainResl<UserCredential>;
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
| Trait in same crate | `` [`TransactionalQuery`] `` |
| Method with path | `` [`run_in_transaction`](crate::domain::query::Transactional::run_in_transaction) `` |
| Type from dependency | `` [`OffsetDateTime`](time::OffsetDateTime) `` |

---

## 5. Layer-specific patterns

### Domain query traits (`src/domain/query/`)

- Describe whether the trait is **read-only** (`&self`) or **mutable**
  (`&mut self`), and the transaction context it belongs to.
- For mutable traits: mention `` **only** inside `Transactional::run_in_transaction` ``.

```rust
/// Mutable persistence contract for [`Member`](crate::domain::model::aggregate::member::Member),
/// used **only** inside a transaction via [`TransactionalQuery`](crate::domain::query::TransactionalQuery).
#[async_trait::async_trait]
pub trait MemberQueryMut {
    /// Inserts a new member row from the creation form.
    async fn create(
        &mut self,
        form: crate::domain::model::aggregate::member::MemberForm,
    ) -> crate::domain::result::DomainResl<crate::domain::model::aggregate::member::Member>;
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
/// Blanket-impl marker: every [`TransactionalQuery`] is a
/// [`UserQeuryMut`](crate::domain::query::user::UserQeuryMut).
trait UserQeuryMut: domain_query::user::UserQeuryMut {}
```

---

## 6. Quick checklist

Before opening a PR with a new or modified trait, verify:

- [ ] Trait has a `///` doc comment.
- [ ] Every method has a `///` doc comment.
- [ ] Methods in multi-method traits are separated by one blank line.
- [ ] Intra-doc links use `[` `]` syntax where possible.
- [ ] `cargo doc` produces no broken-intra-link warnings for the crate.
