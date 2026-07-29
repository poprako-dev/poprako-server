# Rust module dependencies

Internal dependencies in the root crate's hand-written production `src/` must
point downward in the module tree or travel upward to a nearest common ancestor
and then downward into another branch. A module may not depend on one of its
strict ancestors (`MOD001`).

```text
crate::a          -> crate::a::b         allowed: downward
crate::a::b       -> crate::a::shared    allowed: up, then down
crate::a::b       -> crate::a            forbidden: only upward
```

Shared helpers therefore live in a private child of the nearest common
ancestor, such as `a::shared`, with plain-public items where sibling access is
required. They do not live directly in `a` for descendants to pull upward.

The checker analyzes `use`/`pub use`, qualified paths, import aliases, types,
expressions, trait bounds, and source-level attribute paths. It resolves file
and inline modules, then rejects every multi-module strongly connected
component (`MOD002`). Production feature conditions are treated as a strict
union so architecture does not change by feature selection.

An internal symbol used exclusively as the trait or self type in
`impl Trait for Type` is a registration/extension edge and is omitted. If the
same import is used in a signature, where clause, field, or implementation
body, it is an ordinary dependency. An upward glob import cannot prove this
exception and remains forbidden.

Test-only cfg subtrees and the generated Diesel schema at
`src/part_impl/repo/rdb_impl/schema.rs` are excluded. A condition such as
`cfg(any(test, feature = "x"))` remains in scope because it can compile in
production. Macro expansions are not analyzed.

The checker is diagnostic-only and does not provide `--fix`:

```bash
fmt/.venv/bin/python fmt/module-dependency/check.py
fmt/.venv/bin/python fmt/module-dependency/check.py --self-test
```
