# Rust visibility style

Hand-written production Rust under the root crate's `src/` uses either plain
`pub` or no visibility modifier. Restricted forms (`pub(crate)`, `pub(super)`,
`pub(self)`, and `pub(in ...)`) are forbidden (`VIS001`). Put a plain-public
item behind a private module when its effective interface is crate-local.

Struct fields are private outside the `model` and `data` module trees
(`VIS002`). This applies to named and tuple fields. Expose construction and
access through functions instead of widening fields. Fields in `model` and
`data` may use plain `pub`, but restricted visibility remains forbidden.

```rust
mod shared;

// In shared.rs: visible through the private module boundary.
pub fn normalize() {}

// Outside model/data, construction is controlled by the type.
pub struct Service {
    state: State,
}
```

The checker excludes test-only cfg subtrees and the generated Diesel schema at
`src/part_impl/repo/rdb_impl/schema.rs`. A condition such as
`cfg(any(test, feature = "x"))` is still checked because it can exist in a
production configuration. Macro expansions are outside the source-AST scan.

The checker is diagnostic-only and does not provide `--fix`:

```bash
fmt/.venv/bin/python fmt/visibility-style/check.py
fmt/.venv/bin/python fmt/visibility-style/check.py --self-test
```
