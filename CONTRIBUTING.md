# Contributing

PopRaKo uses a two-step integration flow:

1. Open feature and fix pull requests against `dev`.
2. Periodically merge `dev` into `main`. A push to `main` runs required CI and
   deploys production through the protected GitHub Actions environment.

Do not deploy from feature branches or `dev`. Keep pull requests focused and
use conventional commit subjects such as `feat:`, `fix:`, `refactor:`,
`test:`, `docs:`, and `chore:`.

## Required checks

Run the repository entry points directly:

```sh
sh scripts/ci-check.sh
sh scripts/ci-test.sh
sh scripts/ci-openapi-check.sh
sh scripts/ci-typecheck.sh
```

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
