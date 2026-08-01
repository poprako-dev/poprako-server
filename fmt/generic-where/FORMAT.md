# Generic bounds in where clauses

Bounds on type and lifetime parameters must be declared in a `where` clause,
not inline in the generic parameter list. This applies to functions, structs,
enums, traits, type aliases, unions, and `impl` blocks, including declarations
nested in another module or `impl` block.

`impl Trait` in argument position is forbidden, as are fields and type
aliases: give it a named generic parameter and put its bounds in the `where`
clause. Return-position `impl Trait` (an opaque return type) is permitted,
because it cannot be replaced by a named generic parameter without breaking
type inference at every call site.

```rust
// ❌ Forbidden
fn load<T: Loadable>() {}
impl<T: Loadable> Loader<T> {}

// ✅ Required
fn load<T>()
where
    T: Loadable,
{
}

impl<T> Loader<T>
where
    T: Loadable,
{
}

// ❌ Forbidden
fn develop(develop: &(impl EffectDevelop + Sync)) {}

// ✅ Required
fn develop<D>(develop: &D)
where
    D: EffectDevelop + Sync,
{
}

// ✅ Allowed — return-position impl Trait stays opaque
fn iterator() -> impl Iterator<Item = u8> { todo!() }
```

The checker scans production source under `src/`. Modules that are unreachable
without `cfg(test)`, including external test-module files, are masked. A cfg
expression that can also be enabled in production remains in scope.

The checker reports `GEN001` for named inline bounds and `GEN002` for every
argument-position (and field/alias) `impl Trait` occurrence; it does not
modify Rust source:

```bash
uv run fmt/generic-where/check.py
uv run fmt/generic-where/check.py --self-test
```
