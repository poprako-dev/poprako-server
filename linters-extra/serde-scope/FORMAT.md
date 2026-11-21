# serde scope

Keeps HTTP wire concerns in `src/api/http/object` and out of domain layers.
`src/api/http/object` is the only HTTP wire module: request bodies and response
payloads are defined there, while `src/data`, `src/model`, `src/shape`,
`src/complex`, and `src/usecase` stay free of `serde`, `serde_json`, and
`utoipa`.

## Rules

- `SSC001`: `serde`, `serde_json`, and `utoipa` imports, derives, attributes,
  and paths are forbidden in `src/data`, `src/model`, `src/shape`,
  `src/complex`, and `src/usecase`.
- `SSC002`: serde-derived or hand-written `Serialize`/`Deserialize` type
  definitions and impls are allowed only under `src/value/`, `src/util.rs`,
  `src/util/patch_input.rs`, and `src/api/http/object/`. Configuration and
  infrastructure adapters (`src/config`, `src/part`, `src/part_impl`) are
  outside this migration and are ignored. HTTP infrastructure
  (`src/api/http/result.rs` and `src/api/http/shared/metric.rs`) is likewise
  exempt.
- `SSC003`: a serde type definition under `src/api/http/**` must live under
  `src/api/http/object/**`. `src/api/http/result.rs` and
  `src/api/http/shared/metric.rs` are HTTP infrastructure and exempt.
- `SSC004`: a plain `pub` struct, enum, or type alias in `src/value`,
  `src/model`, or `src/data` whose only non-test references outside its
  defining file are under `src/api/**` is API-only. Such a type belongs in
  `src/api/http/object`.
- `SSC005`: `src/api/http.rs` must declare `mod object;`, and no file outside
  `src/api/http` may reference `api::http::object`. The declaration's own
  visibility is not constrained: `pub` stays crate-only because `mod api` is
  private.

Test-only source (`tests.rs`, `tests/`, and `mock` paths) is ignored, matching
the shared production-source mask.

## Diagnostics

```text
src/data/instr/user.rs:5:1: SSC001: serde, serde_json, and utoipa imports are forbidden outside the HTTP wire object module
src/api/http/handler/user.rs:106:1: SSC003: serde wire types under src/api/http must live in src/api/http/object
```

## Baseline

`baseline.txt` records the accepted violations that remain while domains are
migrated one at a time. Each line is `path:CODE`; `#` starts a comment. A
matching `path:CODE` pair suppresses the diagnostic. Regenerate it after each
migration phase and never add a new entry for new code:

```sh
linters-extra/.venv/bin/python linters-extra/serde-scope/check.py \
    --root . --write-baseline
```

New code must satisfy the rules directly; the baseline only shrinks.

## Known limits

- `SSC004` performs source-level identifier matching, not compiler name
  resolution. A same-named type referenced from `src/api/**` can be mistaken
  for a use of the declared type. References are matched by the final path
  segment across production files, and the defining file is excluded.
- Macro-generated code is not expanded. A type whose only use is produced by a
  declarative or procedural macro may be reported as unused or API-only.
- Types re-exported through a module are attributed to the module that names
  them in source, not to the resolved definition site.
- References from test-only files are ignored by design.

## Running

```sh
linters-extra/.venv/bin/python linters-extra/serde-scope/check.py --self-test
linters-extra/.venv/bin/python linters-extra/serde-scope/check.py --root .
sh linters-extra/run-check.sh
```
