---
name: test-spec
description: Apply PopRaKo Rust, PostgreSQL, and HTTP integration test conventions when changing or reviewing tests or externally visible behavior.
---

# Test conventions

- Keep Rust tests in a sibling `tests.rs` or adjacent `tests/` subtree;
  do not append inline test modules after production items.
- Exercise server use-case behavior through
  `part_impl::repo::mock_impl::Mock` and module-local fixtures where practical.
  Other workspace crates use their own contracts and local helpers.
- Add RDB tests for correctness depending on queries, PostgreSQL constraints,
  locking, rollback, or generated schema behavior.
- Cover success and relevant failure classes: invalid input, missing data,
  permission denial, stale state, concurrency, and rollback.
- Use descriptive names and the checked source-comment and layout rules.
- For changed HTTP behavior, update the affected Vitest suite and its case
  documentation under [tests/AGENTS.md](../../../tests/AGENTS.md).
- For changed API contracts, regenerate and verify OpenAPI under
  [docs/AGENTS.md](../../../docs/AGENTS.md).

## Validation

Run from the repository root unless indicated. Root AGENTS specifies the
mandatory Rust checks and the full pre-commit gate.

| Scope | Command |
| --- | --- |
| Workspace tests | `just test` |
| Workspace compilation | `just check` |
| TypeScript static checks | `pnpm run check` from `tests/integration-tests/` |
| OpenAPI consistency | `sh scripts/ci-openapi-check.sh` |

The Rust recipes take no arguments. Never narrow a run with a package, target,
feature, or test filter: a different argument set produces a new artifact
fingerprint and forfeits reuse of the existing `target/` contents.

Feature-gated RDB tests run under `just test`, which uses `--all-features` and
passes no filter. Read the run output and confirm the touched tests actually
executed; a passing suite that never reached them is not evidence.

Database-backed HTTP runs use `sh scripts/api-integration-test.sh`. Follow
the linked tests instructions for its disposable target and root approval
rules. Migration validation uses the dedicated CI database and the full
apply → revert-all → apply cycle through `scripts/ci-migration-check.sh`.
