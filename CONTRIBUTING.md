# Contributing

PopRaKo uses a trunk-based integration flow:

1. Create a short-lived feature or fix branch from `main`.
2. Open its pull request directly against `main` and merge only after all
   required checks pass.
3. A push to `main` reruns required CI and deploys production through the
   protected GitHub Actions environment.

Do not maintain a separate integration branch or deploy from feature branches.
Keep pull requests focused and use conventional commit subjects such as
`feat:`, `fix:`, `refactor:`, `test:`, `docs:`, and `chore:`.

## Required checks

Run the repository entry points directly:

```sh
sh scripts/ci-check.sh
sh scripts/ci-test.sh
sh scripts/ci-openapi-check.sh
sh scripts/ci-typecheck.sh
```

Workspace tests enable all features and require Docker for disposable
PostgreSQL containers. `sh scripts/ci-local.sh` also validates the migration
apply → revert-all → apply cycle and dependency security audit. Main pushes
add HTTP integration through `sh scripts/ci-http.sh` using a dedicated R2 test
bucket in the GitHub `integration` environment. For a local HTTP run, explicitly
supply its disposable database and test-only R2 settings; see [the release
checklist](docs/1.0.0-release-preparation.md).

`just` is optional local convenience and is never required by CI/CD.

Rust 1.95 and the Rust 2024 edition are required. Existing project rules in
`AGENTS.md` and `.agents/skills/` are mandatory for changed code. In
particular, keep new or substantially changed Rust files below 600 lines,
prefer guards or `match` over `if ... else`, and follow the checked-in import,
identifier, error, tracing, and test conventions.

When an externally visible HTTP behavior or integration scenario changes,
update `tests/integration-tests/TESTCASES.md`. Regenerate
`docs/swagger.json` with `cargo run -p poprako-swagger > docs/swagger.json`
when the OpenAPI surface changes.

Schema changes require matching `up.sql` and `down.sql` files. Production
migrations are applied only by CD through `scripts/ga-apply-migrations.sh`.
