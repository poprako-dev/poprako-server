# Debug implementation reduction

Status: implemented and validated.

## Objective

Keep production `Debug` implementations only where logging, trait contracts,
or workspace consumers require them. Remove unused implementations and retain
test-only implementations only when assertions need them.

## Rules

- Preserve production error diagnostics and required trait bounds, including
  `std::error::Error` and Diesel `ToSql`.
- Keep useful, safe instruction fields in use-case spans. Skip whole inputs
  containing private text or credentials and record safe metadata explicitly.
- Follow nested derive dependencies through structs and enums.
- Do not gate production requirements on `debug_assertions`.
- Use `cfg_attr(test, derive(Debug))` for demonstrated unit-test requirements.
  Cross-crate test consumers compile dependencies without `cfg(test)`; review
  those requirements separately.
- Preserve serialization, HTTP contracts, SQL, generated files, and unrelated
  user changes. Do not change linters or run release builds.

## Baseline

The initial static audit found 195 production-source structs with unconditional
`Debug`: 71 with apparent production dependencies and 124 reduction candidates.
These are hypotheses to verify against compiler diagnostics and tests, not a
promise that every candidate can be removed independently. Companion enums
must also be reviewed.

An existing user change in `src/part_impl/obj_dept.rs` is outside this work.

## Execution

- [x] Record the scope and validation plan.
- [x] Replace whole-payload logging for translation imports, terminology
  imports, Unit searches, and text transformations with safe metadata.
- [x] Extend the same treatment to comment and announcement bodies, terminology
  writes, and source-text search filters.
- [x] Remove unused Debug from configuration, HTTP envelopes and queries,
  DTO outputs, internal models, JWT claims, object infrastructure, and helpers.
- [x] Review instruction and value dependencies, including companion enums.
- [x] Restore only demonstrated test or cross-crate requirements.
- [x] Review the complete diff and remaining production Debug implementations.

## Validation

- Run `just fmt`, `just fmt-check`, `just check`, and `just clippy` using the
  repository's unchanged canonical recipes.
- Run server unit tests and affected object-department tests. Inspect their
  feature gates before choosing additional database tests; this task changes
  neither SQL nor database behavior.
- Record results, final counts, and any outstanding limitations below.

## Results

### Implementation

| Production-source structs | Before | After |
| --- | ---: | ---: |
| Unconditional Debug | 195 | 59 |
| Test-only Debug introduced by this change | 0 | 14 |
| Debug removed entirely from previous candidates | 0 | 122 |

The remaining 59 unconditional implementations are 49 safe instruction types,
eight value types required by tracing, `UnitEditLogSummary`, and `ObjKey`.
The reduction exceeds the original estimate because ten logging
boundaries no longer require whole inputs or their nested text types.

`ObjKey` remains unconditional because server-crate tests compare complete
object identities from this dependency. It contains identity/version/storage
key information, not signed URLs or credentials. Unit-test `cfg(test)` in the
server does not enable that flag in the object-department dependency.

`ObjUrls` no longer implements Debug. Two server tests extract the error with
`err().expect(...)` before checking its variant, so unexpected successful
results cannot print signed URLs. Their rejection assertions remain intact.

Companion enums were reviewed as well: ten now derive Debug only for tests.
Production error enums and Diesel's `LocalMessageStatus` retain their required
implementations. No implementation is conditional on `debug_assertions`.

The tracing regression test in `src/test_util/logging_tests.rs` calls four
actual use-case boundaries with private sentinel text and verifies that their
spans retain safe metadata while omitting both `instr` and the sentinel.

### Validation status

- [x] `just fmt` and `just fmt-check`.
- [x] `just check` (workspace, all targets, all features).
- [x] `just clippy` (workspace, all targets, all features, warnings denied).
- [x] `sh scripts/ci-test.sh` (workspace default-feature tests), including the
  private-payload tracing regression test: 524 passed, zero failed.
- [x] `git diff --check`; modified Rust files remain below 600 lines.

No release build, binary-size measurement, database migration, or deployment
was performed. Feature-gated database tests are compiled by the canonical
checks; their execution is outside this SQL-independent change.
