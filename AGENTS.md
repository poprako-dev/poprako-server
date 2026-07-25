# poprako-server - Agent Context

MUST FOLLOW `just deploy-release` WHEN GENERATING RELEASE.

`poprako-server` is the Rust 2024 backend for manga translation project
management. The executable is active: `src/main.rs` wires the production
harness and starts the Axum HTTP server.

`cargo fmt` & `cargo check --all-features` is NECESSARY every time you make edit. But not to run fmt/ unless user ask you to do so.

## Active Architecture

`src/lib.rs` is the authoritative module graph.

```text
src/
├── api/http/       # Axum handlers, middleware, router, OpenAPI, server
├── complex/        # Pure business rules and permission checks
├── data/           # Request Data and response Val DTOs
├── harn.rs         # Production application-harness composition
├── model/          # Persisted application models and input forms
├── part/           # Ports and repository step descriptors
├── part_impl/      # RDB, R2, JWT, prom, effect, and test-mock adapters
├── usecase/        # Generic application orchestration
└── value/          # Shared value objects and enums
```

- `part::repo::step` describes repository operations. `Execute` performs a
  standalone operation; `Advance` performs an operation inside a transaction.
- `Drive::with_context` supplies the transaction context. A use case owns the
  transaction boundary; `complex` remains pure and never drives transactions.
- `Harn` composes production ports. Tests use `part_impl::repo::mock_impl` and
  the test helpers local to the touched module.
- RDB repository code and generated Diesel schema live under
  `src/part_impl/repo/rdb_impl/`. Never edit `schema.rs` directly or import it
  through a `schema::` alias. Change migrations, run `just mgr-schema`, then
  use the generated table module through its full local path.

The Go project under `references/poprako-s/` is a business-behavior reference.
Read it before changing a feature, but do not copy its architecture or mention
its paths in Rust comments.

## Implementation Rules

- For non-trivial work: plan, implement, then review with targeted validation.
- Read nearby active Rust code before choosing names, trait bounds, error
  handling, or test structure.
- Keep comments in English. Do not use `if ... else`; prefer `match`, guards,
  or `let ... else`.
- Keep Rust files under 600 lines and place blank lines between statements.
- Give typed local bindings specific domain names, such as `comic_info` or
  `cover_reservation`.
- Bind a data struct before passing it to a step factory; do not nest inline
  struct literals in step calls.
- Bind transaction output before returning from a use case. For unit output,
  await the transaction, then return `Ok(())`.
- Preserve user-authored working-tree changes. Do not overwrite unrelated edits.

## Active Development Slice

When a behavior changes, update only the layers it needs, in this order:

1. Read current Rust code and the corresponding business reference.
2. Add shared `value`, `model`, or pure `complex` logic where warranted.
3. Add `Data`/`Val` DTOs, repository steps and `XxxRepo<C>` /
   `XxxRepoTransactional<C>` bounds.
4. Implement both RDB and mock adapters as needed.
5. Implement the use case, then the HTTP handler/router/OpenAPI surface when
   the behavior is exposed over HTTP.
6. Add focused Rust tests and keep `tests/integration-tests/TESTCASES.md` in
   sync with integration-suite changes.

## HTTP and Migrations

HTTP routes are active under `/api/v1`; health is `/api/health`. Handlers use
`Accept as _`, propagate use-case errors with `?`, and keep router and
`#[utoipa::path]` declarations aligned. The `swagger` Cargo feature exposes
`/api/swagger-ui` and `/api/openapi.json`; `--swagger` prints the generated
specification.

For schema changes, edit matching migration `up.sql` and `down.sql`, apply or
revert with `just mgr-run` / `just mgr-rev`, regenerate with `just mgr-schema`,
then compile. The generated file is
`src/part_impl/repo/rdb_impl/schema.rs`.

## Project Skills and Verification

Project-local Rust guidance lives in `.agents/skills/`. Load the relevant skill
before editing its area; the active skills cover conventions, errors, traits,
tracing, tests, imports, and formatting.

Run the narrowest useful check first, then broader checks when shared ports,
transactions, API, or error handling change:

```text
cargo fmt --check
cargo check
cargo test -p poprako-server
```
