---
name: trait-def-spec
description: Documentation conventions for active PopRaKo port traits. Use when creating or changing traits under src/part, shared ports, or transactional repository bounds.
---

# Trait Documentation

Every trait and trait method must have an English `///` doc comment that describes its contract, not a particular adapter. Separate method blocks in a multi-method trait with one blank line.

For repository traits, document whether the trait exposes independent `Execute` operations or transaction-bound `Advance` operations. Explain the role of `C` as the context anchor and link to `Drive::with_context` when it helps a reader understand the boundary.

```rust
/// Transactional user repository operations.
///
/// Each operation advances against the shared context supplied by
/// [`Drive::with_context`].
pub trait UserRepoTransactional<C>:
    for<'a> Advance<Create<'a>, C, Error = RegularError>
{
}
```

Prefer valid intra-doc links to nearby public types. Do not copy legacy `Query`, `Aggr`, `DomainResult`, or `RepoTransactional` examples. Keep marker or blanket traits documented as well when they are part of an active contract.
