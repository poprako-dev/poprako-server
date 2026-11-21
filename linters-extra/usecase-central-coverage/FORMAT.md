# Usecase-central coverage

`UCC001` rejects a definition in `part` or `model` that is not reachable from
`usecase` or `complex`. `UCC002` reports an unreadable scanning contract, such
as a missing manifest/library root or malformed Rust in a constrained layer.

The rule encodes the architectural invariant that the lower domain layers exist
only to serve the application core: a port, step/`Oper`, trait, model type,
constant, helper function, or type alias with no consumer in `usecase` or
`complex` does not belong in `part`/`model`.

## Reachability contract

Roots are every reference in production `src/usecase` and `src/complex` files.
A definition becomes reachable when a root reference resolves to it, and then
the definition's own references propagate reachability transitively.

Edges followed from a reachable definition:

- The item body: field types, enum variant payloads, trait supertraits and
  method signatures, type alias values, constants, function signatures.
- Contract attributes declared on the item, such as
  `#[oper(output = Vec<PageSourceUnitDiffStats>)]`. These attributes are the
  generated step contract and are treated as part of the definition.
- `impl` blocks written inside `part` or `model`, matched by the implemented
  type. This links an `Oper` to its `Output`/input model types.
- Imports, including aliases, globs, re-exports, `crate`/`self`/`super`, inline
  modules, and paths reached through `use` chains.
- Plain type and identifier references, so trait bounds in usecase generics
  (`R: UserRepo<C>`) count as references.

Propagation stops at the constrained layers: a model type referenced only by a
`data`, `api`, `extra`, `harn`, `part_impl`, `tests`, or benchmark definition is
**not** reachable, because those layers are consumers rather than roots. In
particular, `part_impl` mock and RDB implementations never make a `part` or
`model` definition used.

## Not consumers

- `part_impl`, `data`, `api`, `extra`, `harn`, `shared`, `shape`, `value`.
- Test and mock paths (`tests.rs`, `test/`, `tests/`, `mock/`, `mock_impl/`)
  and `cfg(test)`-only items, following the shared production-source masker.
- Benchmark-only code.

`complex` and `usecase` are the only roots. The rule does not apply to
`poprako-server` infrastructure crates, which the runner also visits.

## Scope and syntax

- Applies only to the Cargo package named `poprako-server` with `src/lib.rs`.
- Covers `struct`, `enum`, `union`, `trait`, `type`, `const`, `static`, and
  `fn` items at module level in the six layer roots and their descendants,
  including inline modules. Trait and impl associated items are part of their
  parent definition, not separate definitions.
- Parses Rust with Tree-sitter. Comments and string contents are ignored.
  Optional production features remain checked; `cfg(test)` is excluded.
- This is source analysis, not compiler name resolution or macro expansion.
  Paths built entirely by macros/proc-macros, `include!`, and custom
  `#[path]` layouts are not resolved. Normal `foo.rs`, `foo/bar.rs`,
  `foo/mod.rs`, and inline modules use their Rust module location.
- Malformed constrained source fails closed with `UCC002` rather than silently
  skipping the file.

## Diagnostic

```text
src/model/write/page_source.rs:15:1: UCC001: `struct PageSourceEntry` in `model` is not reachable from usecase or complex
```

Fix by deleting the definition or by giving it a real consumer in `usecase` or
`complex`. Moving it into `part_impl` is also valid when the adapter genuinely
owns it.

## Running

From the repository root, using the existing linter environment:

```sh
uv run --no-project --python linters-extra/.venv/bin/python python linters-extra/usecase-central-coverage/check.py --self-test
uv run --no-project --python linters-extra/.venv/bin/python python linters-extra/usecase-central-coverage/check.py --root .
sh linters-extra/run-check.sh
```
