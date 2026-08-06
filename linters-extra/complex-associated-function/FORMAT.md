# Complex associated-function interface

The public module interface rooted at `src/complex.rs` must not export free
functions. Public behavior must be an associated function declared in an
inherent `impl` whose target type name ends in `Complex`.

```rust
// ❌ Forbidden — complex modules do not export free functions.
pub fn normalize() {}

// ❌ Forbidden — public functions cannot be attached to arbitrary types.
impl Normalizer {
    pub fn normalize() {}
}

// ✅ Required — public behavior belongs to a Complex type.
pub struct TextComplex;

impl TextComplex {
    pub fn normalize() {}
}
```

Private helper modules may use free functions, including plain `pub` functions
that are visible across the private module boundary. They fail only when a
public complex module declares or re-exports them. The checker scans production
Rust sources under `src/complex/`, excluding test-only module files through the
shared production-source masker.

## Checker

```bash
uv run fmt/complex-associated-function/check.py --self-test
uv run fmt/complex-associated-function/check.py
```
