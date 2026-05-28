# poprako-r — Agent Context

`poprako-r` is a **Rust rewrite** of `poprako-s`, an event-driven backend service
for managing manga (comic) translation projects. It handles teams, worksets,
comics, chapters, pages, translation units, assignments, and user/member
management.

- **Runtime**: Tokio async
- **Web framework**: Salvo
- **ORM**: Diesel (async via `diesel-async`)
- **Database**: PostgreSQL

---

## Reference: Original Go Project

The original `poprako-s` Go implementation lives at **`references/poprako-s/`**.
When implementing or modifying a feature, **always read the corresponding Go
code first** to understand the business logic, domain invariants, and intended
behavior. The Rust project should faithfully reproduce the domain model and
business rules from the Go version.

Key reference files:

| Area | Go path |
|------|---------|
| Architecture overview | `references/poprako-s/AGENTS.md` |
| Domain models | `references/poprako-s/internal/domain/model/` |
| Repository interfaces | `references/poprako-s/internal/domain/repo/` |
| Use-case implementations | `references/poprako-s/internal/app/impl/` |
| Infrastructure (GORM repos) | `references/poprako-s/internal/infra/repo/` |
| Workflow state machine | `references/poprako-s/internal/domain/model/workflow.go` |
| Event definitions | `references/poprako-s/internal/domain/event/` |
| Migrations (SQL) | `references/poprako-s/migrations/` |

---

## Project Skills

Three project-local skills define coding conventions for specific layers:

- **`poprako-conventions`** — Infra/query layer (Diesel) conventions
  (`src/infrastructure/query/`)
- **`poprako-aggr-conventions`** — Domain aggregate layer conventions
  (`src/domain/model/aggregate/`)
- **`thirdparty-macro-usage-spec`** — Fully-qualified paths for third-party macros
  (`tracing::instrument`, `serde::Serialize`, etc.)
- **`tracing-usage-spec`** — Where `#[tracing::instrument]` should and should
  NOT be placed (no constructors, no domain pure logic)

These skills are in `.pi/skills/` and are auto-triggered when working in the
corresponding source paths.

---

## Project Structure

```
src/
├── main.rs                  # Entry point
├── lib.rs                   # Library root
├── api/                     # Salvo HTTP handlers / routes
│   └── *.rs
├── domain/
│   ├── model/
│   │   └── aggregate/       # Domain aggregates (User, Team, Chapter, ...)
│   └── query/               # Repository trait interfaces
├── usecase/                 # Application use-case orchestration
├── infrastructure/
│   ├── query/               # Diesel repository implementations
│   │   └── entity/          # Diesel entity structs (Rows, Entries, Aspects)
│   └── ...
└── util.rs                  # Shared utilities
```
