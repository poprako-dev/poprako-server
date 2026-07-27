# Rust use style

Every file and inline module keeps its direct items in this order: ordinary
`use` declarations, `pub use` declarations, `mod` declarations, then all other
code. This includes `#[cfg(test)] mod tests;`; test modules may not remain at
the end of a file after functions, traits, implementations, or other code.

Ordinary `use` and `pub use` declarations form separate blocks with exactly one
blank line between them. Each block is independently grouped in this order:
`super`, `std`, third-party crates, local workspace crates, then `crate`/`self`.
Adjacent non-empty groups have exactly one blank line between them. Local
imports inside functions remain outside this rule.

Brace lists may contain only direct leaves. Imports with the same leaf prefix
are merged, and duplicate imports are removed. Known trait imports use `as _`
unless the trait is explicitly named in a type-level position.

`#[cfg(...)]` attributes define independent import groups. The checker never
merges or requires group-separator blank lines across different conditions.
`--fix` only combines declarations that have identical full attribute lists,
so it cannot broaden an attribute's effect. It also moves complete declaration
chunks, including outer attributes and documentation comments, into canonical
item order and iterates until import grouping and item ordering both converge.

```bash
# Check imports without rewriting source. This is what `just fmt-check` runs.
uv run fmt/use-style/check.py

# Canonicalize imports through the Rust AST, then run `cargo fmt --all`.
uv run fmt/use-style/check.py --fix
uv run fmt/use-style/check.py --self-test
```
