# poprako-r — Agent Context

`poprako-r` is a **Rust rewrite** of `poprako-s`, an event-driven backend service
for managing manga (comic) translation projects. It handles teams, worksets,
comics, chapters, pages, translation units, assignments, and user/member
management.

Rust-idiomatic code style over simply copying Go code is perferred(or even enforce to apply).

- **Runtime**: Tokio async
- **Web framework**: Axum
- **ORM**: Diesel (async via `diesel-async`)
- **Database**: PostgreSQL

---

## Reference: Original Go Project

The original `poprako-s` Go implementation lives at **`references/poprako-s/`**.
When implementing or modifying a feature, **read the corresponding Go code
first** to understand the business logic, domain invariants, and intended
behavior. The Go project is a business-reference source, not an implementation
template. PopRaKo-R may use different names, route shapes, handler mechanics,
error handling, and ordering behavior as long as the intended Rust behavior is
functionally correct.

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

---

## Project Skills

Project-local skills define coding conventions for the project and specific
layers:

- **`general-conventions`** — Project-wide Rust coding conventions
- **`query-domain-spec`** — Domain query trait conventions
  (`src/domain/query/`)
- **`query-infra-spec`** — Infra/query layer (Diesel) conventions
  (`src/infra/query/`)
- **`poprako-aggr-conventions`** — Domain aggregate layer conventions
  (`src/domain/model/aggr/`)
- **`thirdparty-macro-usage-spec`** — Fully-qualified paths for third-party macros
  (`tracing::instrument`, `serde::Serialize`, etc.)
- **`tracing-usage-spec`** — Where `#[tracing::instrument]` should and should
  NOT be placed (no constructors, no domain pure logic)
- **`trait-def-spec`** — Documentation and formatting conventions for all
  `trait` definitions (doc comments, blank lines between methods)
- **`error-handling-spec`** — Error definition and handling philosophy across
  all layers
- **`api-http-spec`** — Axum HTTP handler, router, utoipa OpenAPI, and Swagger
  conventions (`src/api/http/`)

These skills are in `.pi/skills/` and are auto-triggered when working in the
corresponding source paths.

---

## Conventions

- **Learn conventions from existing layer code**: Before modifying or adding code in a layer (e.g., `src/usecase/`, `src/infra/query/`, `src/api/http/handler/`, `src/domain/`), first **read the existing implementations in the same layer** to understand the actual conventions used — import style, path qualification, error handling patterns, calling conventions, and struct/function idioms. The existing code is the source of truth for how that layer should be written. Do not rely solely on skill files or general expectations.
- **English-only comments**: All comments (line `//`, block `/* */`, and doc `///` / `//!`) must be written in English. No other language is permitted in source code comments.
- **No Go references in comments**: Source code comments must not reference Go counterparts (e.g., file paths, function names, or patterns from `references/poprako-s/`). Comments describe what the Rust code does, not how the Go version does it.
- **Never modify user-authored changes**: The user may have already modified files or is in the process of modifying them. Never overwrite, revert, or undo changes that the user has made. If a change the user made conflicts with a convention or new requirement, flag it explicitly and ask before modifying. Never use `git checkout` or `git reset` on files that the user has edited in the working tree.
- **Fully-qualified local variable names for models**: Every `let` binding that holds a typed domain value (model, DTO, form, update-payload, reservation, spec, etc.) MUST carry both the domain name and the type suffix. **Never** use bare short names.
  - `let comic_info` not `info`; `let comic_form` not `form`; `let comic_info_vals` not `values`.
  - `let workset_info` not `info`; `let workset_info_update` not `update`; `let workset_infos` not `infos`.
  - `let team_info` not `info`; `let team_form` not `form`.
  - `let system_mail_infos` not `mails` or `system_mails`; `let system_mail_info` not `system_mail`.
  - `let cover_reservation` not `reservation`; `let avatar_reservation` not `reservation`.
  - `let mail_list_spec` not `spec`.
  - Shadowed variables in closures (`let repo = repo.transactional().await`) are exempt — they shadow a parameter of the same name for type-state purposes. Use method-call syntax (not UFCS) when the trait is in scope.
- **No struct literals in Step call sites**: Steps must be constructed via their factory functions (`ComicStep::create(...)`, `TeamStep::update_info(...)`, etc.) at the `advance`/`execute` call site. Arguments passed to step factory functions must be **named variables** or individual scalar values — **never** inline struct literals (e.g., `&ComicInfoUpdate { ... }` is forbidden). Construct the data struct as a `let` binding first, then pass a reference to the step function.
  - ✅ `let comic_info_update = ComicInfoUpdate { ... }; repo.execute(&ComicStep::update_info(&comic_info_update))`
  - ❌ `repo.execute(&ComicStep::update_info(&ComicInfoUpdate { ... }))`

---

## Project Structure

```
src/
├── main.rs                  # Entry point
├── lib.rs                   # Library root
├── api/                     # Axum HTTP handlers / routes
│   └── *.rs
├── domain/
│   ├── model/
│   │   └── aggr/            # Domain aggregates (User, Team, Chapter, ...)
│   └── query/               # Repository trait interfaces
├── usecase/                 # Application use-case orchestration
├── infra/
│   ├── query/               # Diesel repository implementations
│   │   ├── entity/          # Diesel entity structs (Rows, Entries, Aspects)
│   │   └── schema.rs        # Generated by `diesel print-schema`
│   └── ...
└── util.rs                  # Shared utilities
```

---

## HTTP API Workflow

When implementing or modifying API HTTP code, follow
`docs/how-to-implement-api-http.md`.

- Import `Accept as _` and use `.accept(...)` for successful handler responses.
- Propagate usecase errors directly with `?`.
- Use standard RESTful routes with plural resource nouns and path params.
- Keep `#[utoipa::path(...)]`, `src/api/http/openapi.rs`, and router paths in
  sync.
- Expose Swagger/OpenAPI only in debug builds under `/api/swagger-ui` and
  `/api/openapi.json`, not under `/api/v1`.
- Do not copy Go API code; use it only to understand business intent.

---

## Database Migration Workflow

When modifying the database schema (e.g., adding a new column to an existing
table), follow these steps in order:

1. **Edit the latest migration's `.up.sql`** in `migrations/<timestamp>_<name>/`
   (and `.down.sql` if needed).
2. **Revert the latest migration** with `just mgr-rev`.
3. **Re-apply the migration** with `just mgr-run`.
4. **Regenerate `schema.rs`** with `diesel print-schema > src/infra/query/schema.rs`.
5. **Update Rust source files** that reference the changed table to match
   the new schema (entity structs, infra query code, domain aggregates).
6. **Run `cargo check`** to verify compilation.

> `just mgr-rev` rolls back the last applied migration. If multiple
> migrations were applied since the target table was last modified, revert
> them one by one until the target migration is undone, then re-run them
> all with `just mgr-run`.

Available `just` commands:

| Command          | Description                                                      |
| ---------------- | ---------------------------------------------------------------- |
| `mgr-run`        | `diesel migration run` — apply pending migrations                |
| `mgr-rev`        | `diesel migration revert` — roll back the last migration         |
| `mgr-add <name>` | `diesel migration generate <name>` — create a new migration pair |
| `mgr-setup`      | `diesel database setup` — create the database (one-time)         |
