# Strict workspace lint policy

The root `Cargo.toml` defines Rust and Clippy policy for all seven workspace
packages. Each package exclusively inherits it with `[lints] workspace = true`.

Previously, crate roots declared `style`, `pedantic`, and `nursery` as `warn`,
allowed three individual rules globally, and the server disabled entire lint
groups under `cfg(test)`. Crate-local attributes also did not cover independent
binary, integration-test, or benchmark crate roots. `-D warnings` promoted active
warnings during CI, but it did not enable disabled rules or undo explicit allows.

Rust warnings are now denied and unsafe code is forbidden. Clippy `all`,
`pedantic`, and `nursery` are denied, together with the selected restriction
rules in the manifest. Group priority is `-1` so individual policy entries have
explicit precedence. The complete `restriction` and `cargo` groups are not
enabled: they include opinionated or mutually incompatible requirements and
third-party dependency constraints unrelated to this project's source policy.

Necessary exceptions name individual rules and include English reasons:

- Test fixture and assertion modules may allow `unwrap_used` and `expect_used`.
  Explicit failure or panic-injection scenarios use function-scoped expectations.
- Locally awaited generic interfaces retain their existing Send/Sync contracts.
- Explicit formatting arguments follow the repository's formatting rules.
- Success constructors retain their shared Result contracts and type inference.
- Diesel changesets and generated serialization/schema implementations retain
  their typed nullable-field and derive semantics.
- The Swagger exporter may write its generated document to standard output.

No crate-wide or test-mode Clippy group exemptions remain. Function-scoped
`expect` attributes also fail CI when their exception becomes unnecessary.

## Validation

Canonical local commands remain:

```sh
just fmt-check
just check
just clippy
```

Clippy checks every workspace package, target, and feature:

```sh
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

`sh scripts/ci-lint-policy.sh` checks actual Cargo workspace membership,
inheritance, required lint levels, group priorities, and global policy weakening.
Seven regression tests cover missing inheritance, member overrides, rule
downgrades, missing rules, priorities, and new global exemptions.

`sh scripts/ci-local.sh` runs the full checked-in CI chain: Rust checks and tests,
repository lint, deployment-script checks, OpenAPI comparison, TypeScript checks,
disposable database migration apply/revert-all/apply, and dependency audit.
Production image builds depend on all four GitHub Actions validation jobs.

Verified on 2026-10-05 with the pinned Rust 1.95.0 toolchain: all three canonical
Rust checks and the full `ci-local.sh` chain passed on the final source. Repository
linters and generated schema/OpenAPI files were not modified.
