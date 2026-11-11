# Minimum public visibility

Every plain `pub` must be justified by a reference from outside its defining
module. A private item remains visible to its own module and that module's
descendants, so `pub` is unnecessary when all uses stay inside that subtree.
The checker covers production Rust in every workspace crate and all references
in package `src`, `tests`, `benches`, and `examples` sources.

## Diagnostics

- `PUB001`: a module, item, or `pub use` re-export is not referenced outside
  its defining module.
- `PUB002`: no field of a struct is used outside its defining module. Fields
  are judged as one unit: one cross-module field reference justifies `pub` on
  every field of that struct, matching the uniform field visibility rule.
- `PUB003`: an inherent associated function, constant, or type is not used
  outside the module that owns the `impl`.

Remove `pub` from the reported declaration. Use `pub(crate)` or another
restricted visibility only when the separate shared `visibility-style` rule
also permits it; this checker does not introduce those forms.

## What counts as a use

- Absolute, relative, imported, aliased, grouped, and wildcard paths.
- Cross-crate paths, including references from bins, integration tests,
  benches, examples, and other workspace packages.
- Field accesses, struct literals, patterns, tuple indexes, and method calls.
- Paths and field names inside macro token trees, because a macro body expands
  where it is invoked.
- Paths written in attribute string arguments, such as
  `deserialize_with = "duration::deserialize"`.
- Signatures of public declarations that themselves require `pub`: a type name
  used by such a signature must remain public to avoid a private-interface
  error.
- Modules whose source contains a module-level macro invocation are treated as
  macro-generated and are not reported, because their child items cannot be
  recovered from syntax alone.

## Exemptions and limits

- Procedural-macro entry points (`#[proc_macro]`, `#[proc_macro_derive]`, and
  `#[proc_macro_attribute]`) must stay `pub`.
- Test-only paths are analyzed for references but do not receive diagnostics.
- This is source analysis, not compiler name resolution or macro expansion.
  Paths built entirely inside opaque procedural-macro output, `include!`, and
  custom `#[path]` layouts may be invisible. Such gaps can only cause a
  missing reference, and the resulting `pub` is kept.
- Generated `schema.rs` remains outside hand-authored source and is not
  reported by this rule.

## Running

```sh
linters-extra/.venv/bin/python linters-extra/minimum-pub/check.py --self-test
linters-extra/.venv/bin/python linters-extra/minimum-pub/check.py --root .
sh linters-extra/run-check.sh
```

The runner invokes this checker once per workspace and runs its isolated
self-test before the shared read-only analysis session.
