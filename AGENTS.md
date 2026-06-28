# poprako-r - Agent Context

`poprako-r` is a Rust rewrite of `poprako-s`, an event-driven backend service
for manga translation project management. It covers users, teams, members,
member invitations, worksets, comics, chapters, pages, units, assignments,
system mail, announcements, and comments.

The current codebase has moved away from the old `domain/query/infra/api`
vertical architecture. The active library entry point is `src/lib.rs`, and the
primary architecture is now a ports-and-transaction-steps application core.

- **Language**: Rust 2024
- **Runtime**: Tokio async
- **HTTP framework dependency**: Axum, currently not wired by `main.rs`
- **ORM dependency**: Diesel async, current generated schema at
  `src/infra/repo/schema.rs`
- **Database**: PostgreSQL
- **Internal workspace crates**:
  - `poprako-transactional` - transaction driver, `Step`, `Execute`, `Advance`
  - `poprako-util` - shared utility/i18n support
  - `poprako-macro` - project macros

Rust-idiomatic code is preferred over copying the Go implementation directly.

---

## Reference: Original Go Project

The original `poprako-s` Go implementation lives at `references/poprako-s/`.
When implementing or modifying a feature, read the corresponding Go code first
to understand business rules, invariants, and intended behavior. The Go project
is a business reference, not an implementation template.

Key reference files:

| Area                        | Go path                                                  |
| --------------------------- | -------------------------------------------------------- |
| Architecture overview       | `references/poprako-s/AGENTS.md`                         |
| Domain models               | `references/poprako-s/internal/domain/model/`            |
| Repository interfaces       | `references/poprako-s/internal/domain/repo/`             |
| Use-case implementations    | `references/poprako-s/internal/app/impl/`                |
| Infrastructure (GORM repos) | `references/poprako-s/internal/infra/repo/`              |
| Workflow state machine      | `references/poprako-s/internal/domain/model/workflow.go` |
| Event definitions           | `references/poprako-s/internal/domain/event/`            |
| Migrations (SQL)            | `references/poprako-s/migrations/`                       |

Do not mention Go paths or Go implementation details in source comments.

---

## Current Architecture

The active crate modules are declared in `src/lib.rs`:

```text
src/
├── lib.rs              # Active library root
├── main.rs             # Stub main; legacy server startup is commented out
├── config.rs           # Application configuration
├── result.rs           # Root error/result layer
├── util.rs             # Shared utilities
├── forward_ref.rs      # ForwardRef helper
├── model/              # Persisted business entity models
├── value/              # Smaller domain value objects/enums
├── complex/            # Pure business/domain logic helpers
├── data/               # Inbound Data and outbound Val DTOs
├── part/               # Port traits: repo, image, auth, prom, effect
├── part_impl/          # Concrete port implementations; currently test mocks
└── usecase/            # Free-standing generic application use cases
```

### Layer Roles

- `model`: persisted business entities and forms/updates that carry storage-ish
  values such as IDs, versions, flags, and `OffsetDateTime`.
- `value`: focused domain value types, enums, and small typed concepts shared by
  models/use cases.
- `complex`: pure business logic that can coordinate model/value rules without
  performing I/O.
- `data`: external-facing DTO layer. Request payloads end with `Data`; response
  values end with `Val`. This layer converts model timestamps to Unix
  milliseconds and resolves signed image URLs when needed.
- `part`: port contracts used by the application core. The important ports are
  `repo`, `image`, `auth`, `prom`, and `effect`.
- `part::repo::step`: repository operation descriptors. Each step implements
  `poprako_transactional::step::Step` and is constructed through domain factory
  types such as `UserStep`, `TeamStep`, `ComicStep`, etc.
- `usecase`: application orchestration. Public use cases are free functions,
  generic over their ports, and compose `Execute`, `Advance`, and `Drive`.
- `part_impl`: adapters for ports. At the moment the committed active
  implementations are test mocks behind `#[cfg(test)]`; old concrete adapters
  remain as backup files under legacy folders.

### Transaction Model

Repository operations are expressed as `Step` values.

- Non-transactional operations use `part::repo::Execute<S>`. Each call owns its
  connection/session and commits independently.
- Transactional operations use `poprako_transactional::advance::Advance<S, C>`
  inside `Drive::with_context(...)`. All steps in the closure share context `C`
  and commit or rollback atomically.
- Each repository domain has a non-transactional trait and a transactional trait:
  `XxxRepo<C>` and `XxxRepoTransactional<C>`.
- The `C` generic parameter is a type-system anchor connecting a repository to
  its transactional handle. It should not be threaded into method signatures
  unless the existing local pattern already does so.
- Use `part::repo::map_drive_err` when mapping transaction-driver errors to
  `RootError`.

Side effects and deferred work must be coordinated through the relevant ports
(`Prom`, `EffectDevelop`, etc.) and should be emitted only after the transaction
semantics are correct.

### Legacy And Backup Code

Several old architecture directories still exist, mostly as `.bak` files:

```text
src/api/
src/domain/
src/infra/
src/usecase_legacy/
```

Treat them as historical reference unless a task explicitly asks to revive or
modify the legacy HTTP/Diesel stack. Do not add new active code to the old
`domain/query` or `infra/query` shape. The old `main.rs` server startup is
commented out and `fn main() {}` is currently the active binary entry point.

---

## Project Skills

Project-local skills are in `.agents/skills/`. Use the relevant skill before
editing Rust code in its area.

Important active skills include:

- `general-conventions` - project-wide Rust conventions
- `code-style-check` - aggregate style checks
- `format-output-spec` / `no-inline-format` - format string rules
- `thirdparty-macro-usage-spec` - third-party macro import/call-site rules
- `tracing-usage-spec` - where `#[instrument]` belongs
- `trait-def-spec` - trait documentation conventions
- `error-handling-spec` - error construction and propagation rules
- `test-spec` - test naming and structure
- `harness-spec` - old harness conventions when touching legacy harness code
- `rust-use-style` and `rust-ident-style` - strict Rust import/path style

Some older skills still mention `src/domain/query`, `src/infra/query`, or
`src/domain/model/aggr`. Treat those paths as legacy unless the current task is
explicitly about that backup architecture. For active code, first follow the
current modules under `model`, `value`, `complex`, `data`, `part`, `part_impl`,
and `usecase`, then apply the project-wide style rules.

---

## Conventions

- **Plan -> execute -> review**: For non-trivial changes, make the local plan
  explicit, implement, then verify with targeted checks.
- **Learn from nearby code first**: Before editing a layer, read existing files
  in the same current layer. The local module is the source of truth for naming,
  imports, path qualification, error handling, and test style.
- **English-only comments**: All source comments and doc comments must be in
  English.
- **No Go references in comments**: Comments describe Rust behavior, not the Go
  source that inspired it.
- **No `if else` control flow**: Do not write `if ... else ...` branches in Rust
  source. Use `match` or guard clauses instead. `let ... else` is allowed.
- **Blank line between statements**: Keep a blank line between any two Rust
  statements. Dense statement blocks are unacceptable.
- **Rust file length limit**: Rust source files must stay under 600 lines. Split
  modules or extract helpers before a file reaches that size.
- **Complex must not own transaction execution**: `Complex` pure helpers must
  not depend on repository transactional traits, `Advance`, or prom
  transactional ports. Permission helpers belong in `XxxPermComplex` and expose
  only `can_*` functions publicly; shared private checks use proxy execute.
  Transactional cleanup and prom image deletion stay in usecase transaction
  flows unless an existing reviewed module explicitly requires otherwise.
- **Never modify user-authored changes**: The working tree may contain user
  edits. Do not revert, overwrite, or "clean up" unrelated changes. If a user
  change conflicts with the task, flag it before editing.
- **Typed local names must be specific**: A `let` binding that holds a typed
  domain value, DTO, form, update payload, reservation, or spec must include the
  domain name and type suffix.
  - Use `comic_info`, not `info`.
  - Use `comic_form`, not `form`.
  - Use `comic_info_vals`, not `values`.
  - Use `workset_info_update`, not `update`.
  - Use `system_mail_infos`, not `mails`.
  - Use `cover_reservation`, not `reservation`.
  - Shadowed variables for type-state transitions, such as
    `let repo = repo.transactional().await`, are allowed.
- **No inline struct literals in step call sites**: Construct data structs as
  named `let` bindings before passing references to step factory functions.
  - Good: `let comic_info_update = ComicInfoUpdate { ... };`
  - Good: `repo.execute(&ComicStep::update_info(&comic_info_update))`
  - Bad: `repo.execute(&ComicStep::update_info(&ComicInfoUpdate { ... }))`
- **No direct `Drive::with_context` tail returns**: Usecase functions must bind
  and destructure transaction output before returning. For unit output, do not
  bind a meaningless value; run the transaction with `?`, then return
  `accept(())`.
- **Unused parameters**: Delete unused parameters when the signature is under
  project control. If a trait/framework signature forces the parameter, use bare
  `_`, not semantic underscore-prefixed names.

---

## Current Implementation Workflow

When adding or changing a behavior in the active architecture:

1. Read the corresponding Go business code and the current Rust files in the
   same domain.
2. Add or update `value` types only when the concept is shared or deserves a
   named domain type.
3. Add or update `model` structs for persisted business state.
4. Put pure cross-model rules in `complex`.
5. Add inbound `Data` and outbound `Val` DTOs in `data`.
6. Add repository step descriptors under `part/repo/step/<domain>.rs`.
7. Add or update `XxxRepo<C>` and `XxxRepoTransactional<C>` traits under
   `part/repo/<domain>.rs`.
8. Add mock behavior in `part_impl/repo_mock/<domain>.rs` for usecase tests.
9. Implement the usecase as a free function under `usecase/<domain>.rs`.
10. Add focused usecase tests using the mock ports.
11. Run formatting, targeted tests, and `cargo check` when feasible.

Prefer small vertical slices. Avoid resurrecting the old harness/API/infra stack
unless that is the explicit task.

---

## HTTP API Status

The HTTP API files are currently backup files under `src/api/**.bak`, and the
active `main.rs` does not start an Axum server. `docs/how-to-implement-api-http.md`
describes the old HTTP workflow and should be treated as guidance only when the
task is specifically to restore or rework HTTP.

If HTTP is re-enabled:

- Keep handlers, router paths, and OpenAPI annotations in sync.
- Import `Accept as _` and use `.accept(...)` for successful responses where the
  active result pattern requires it.
- Propagate usecase errors directly with `?`.
- Use plural REST resource nouns and path params.
- Expose Swagger/OpenAPI only in debug builds under `/api/swagger-ui` and
  `/api/openapi.json`, not under `/api/v1`.

---

## Database Migration Workflow

When modifying the database schema:

1. Edit the relevant migration `.up.sql` and `.down.sql`.
2. Revert the affected migration with `just mgr-rev` as needed.
3. Re-apply migrations with `just mgr-run`.
4. Regenerate schema with `just mgr-schema`.
5. Update Rust source files that reference the changed table.
6. Run `cargo check`.

The generated Diesel schema path is:

```text
src/infra/repo/schema.rs
```

Available `just` commands:

| Command          | Description                                        |
| ---------------- | -------------------------------------------------- |
| `mgr-run`        | `diesel migration run`                             |
| `mgr-rev`        | `diesel migration revert`                          |
| `mgr-reset`      | `diesel migration revert -a`                       |
| `mgr-add <name>` | `diesel migration generate <name>`                 |
| `mgr-setup`      | `diesel database setup`                            |
| `mgr-list`       | `diesel migration list`                            |
| `mgr-schema`     | `diesel print-schema > src/infra/repo/schema.rs`   |
| `style`          | Run all project style checks through Bun scripts   |

---

## Verification

Common checks:

```text
cargo fmt
cargo check
cargo test -p poprako-r
just style
```

Use targeted tests first when touching one domain. Run broader checks when the
change affects shared ports, transaction behavior, result/error handling, or
cross-domain use cases.
