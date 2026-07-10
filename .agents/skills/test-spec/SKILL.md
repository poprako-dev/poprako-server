---
name: test-spec
description: Test structure and documentation rules for active PopRaKo Rust and HTTP integration tests. Use whenever adding or changing tests.
---

# Test Conventions

Keep Rust unit and usecase tests beside the module they verify. Exercise public behavior through current mock adapters where practical; add RDB tests when correctness depends on Diesel, PostgreSQL constraints, locking, or transaction behavior.

For each public usecase behavior, include a positive case and at least one relevant negative case: invalid data, missing resource, permission denial, concurrency, or rollback. Use descriptive test names and retain any nearby project-specific comment format.

HTTP integration tests live under `tests/integration-tests/src/suites/`. When a suite file is added, removed, renamed, or its covered cases change, update `tests/integration-tests/TESTCASES.md` in the same change. Keep case IDs stable unless the case itself is removed.

Validate proportionally:

```text
cargo test -p poprako-server <module-or-test-filter>
cargo test -p poprako-server
cd tests/integration-tests && pnpm typecheck
```

Run the API suite through `scripts/api-integration-test.sh` only with its dedicated integration database configured; it creates and drops that database.
