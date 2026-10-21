# Horizontal layer dependencies

`LYR001` rejects explicitly forbidden dependency edges in the `poprako-server`
package. `LYR002` reports an unreadable scanning contract, such as a missing
manifest/library root or malformed Rust in a constrained layer.

## Forbidden edges

Each row names forbidden target modules. It is **not an allowlist**. There are
no layer ranks, inferred restrictions, or checks requiring every dependency to
appear in a list of known legal edges.

| Source layer | Forbidden target layers |
| --- | --- |
| `value` | `model`, `complex`, `data`, `part`, `usecase`, `api`, `extra`, `part_impl`, `harn`, `shared` |
| `model` | `complex`, `data`, `part`, `usecase`, `api`, `extra`, `part_impl`, `harn`, `shared` |
| `complex` | `data`, `part`, `usecase`, `api`, `extra`, `part_impl`, `harn`, `shared` |
| `data` | `complex`, `part`, `usecase`, `api`, `extra`, `part_impl`, `harn`, `shared` |
| `part` | `complex`, `data`, `usecase`, `api`, `extra`, `part_impl`, `harn`, `shared` |
| `usecase` | `api`, `extra`, `part_impl`, `harn`, `shared` |

`shared` is the server's concrete RDB context infrastructure. Core layers use
the abstractions in `part` instead. `harn` is composition wiring. Delivery
adapters in `api`, `extra`, and `part_impl` may call use cases; this checker
does not impose a reverse restriction on them.

`result`, `util`, `config`, external crates, and future modules are not forbidden.
New submodules inside permitted layers also require no configuration change.
Same-layer dependencies remain outside this check. The existing shared
`module-dependency` checker continues to check module cycles and ancestors.

For example, `complex -> model::artifact::new_type` and `complex -> future`
pass without editing the linter. `complex -> data::new_dto` fails because the
existing `complex -> data` edge is forbidden, regardless of the DTO name.

## Scope and syntax

- Applies only to the Cargo package named `poprako-server`; infrastructure
  packages visited by `run-check.sh` are skipped.
- Covers the six layer root files, their descendants, and inline modules.
- Parses Rust with Tree-sitter, including nested/grouped `use`, `pub use`,
  aliases, wildcards, `crate`/`self`/`super`, crate-name paths, raw identifiers,
  qualified types/calls, impl headers, and explicit paths in macro/attribute
  token trees. Source diagnostics include file, line, column, and the edge.
- Follows resolvable import aliases and re-exports. Re-exporting a legal type
  from a forbidden layer does not make importing that layer legal.
- Comments and string contents are ignored. Production-only scope follows
  the existing extra linters: test/mock paths and `cfg(test)`-only items are
  excluded. Optional production features remain checked.
- This is source analysis, not compiler name resolution or macro expansion.
  Paths constructed entirely by macros/proc-macros, `include!`, and custom
  `#[path]` file layouts are not resolved. Normal `foo.rs`, `foo/bar.rs`,
  `foo/mod.rs`, and inline modules use their Rust module location.

## Running

From the repository root, using the existing linter environment:

```sh
uv run --no-project --python linters-extra/.venv/bin/python python linters-extra/layer-dependency/check.py --self-test
uv run --no-project --python linters-extra/.venv/bin/python python linters-extra/layer-dependency/check.py --root .
sh linters-extra/run-check.sh
```

`run-check.sh` runs this checker's self-tests, then checks workspace package
roots using its existing discovery. The existing CI repository-lint job already
invokes that runner. No Rust lint levels, shared submodule, or Cargo settings
need to change.
