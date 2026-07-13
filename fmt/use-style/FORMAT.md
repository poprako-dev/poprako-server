# Rust use style

Module-scope `use` declarations are grouped in this order: `super`, `std`,
third-party crates, local workspace crates, then `crate`/`self`. Adjacent
non-empty groups have exactly one blank line between them. `pub use` and local
imports inside functions are outside this rule.

Brace lists may contain only direct leaves. Imports with the same leaf prefix
are merged, and duplicate imports are removed. Known trait imports use `as _`
unless the trait is explicitly named in a type-level position.

`#[cfg(...)]` attributes define independent import groups. The checker never
orders, merges, or requires blank lines across different conditions. `--fix`
only combines declarations that have identical full attribute lists, so it
cannot broaden an attribute's effect.

```bash
uv run fmt/use-style/check.py
uv run fmt/use-style/check.py --fix
uv run fmt/use-style/check.py --self-test
```
