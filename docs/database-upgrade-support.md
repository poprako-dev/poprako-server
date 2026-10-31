# Database upgrade support for 1.0.0

The current `migrations/` baseline and its genuine paired rollback files remain
active. CI validates apply, repeated production forward replay, revert-all,
empty business schema/history, and apply again on disposable `db_poprako_ci`.
Application startup never applies migrations. Production forward replay does
not replace an explicit conversion when an existing table has an older shape.

Git history replacement for 1.0.0 does not change database history. Before the
single-commit snapshot is finalized, reconcile the target's recorded versions
with retained migration sources and preserve any missing genuine rollback
sources. No production upgrade completion is inferred from files in this repo.

## Upgrade inventory

| Entry | Source and target | Status and verification |
| --- | --- | --- |
| `migrations/` | Empty database → current baseline; existing compatible schema → idempotent replay | Active. `sh scripts/ci-migration-check.sh` on authorized disposable CI DB |
| `scripts/chapter-admin-upgrade/` | Historical Chapter administration shape in `tests/source.sql` → administration shape required by current baseline | Retained supported conversion under fixture coverage. CI applies the preservation fixture and upgrade twice; production execution status must be checked separately |
| `scripts/migrate-review-editor.sql` | Review storage variants represented by `tests/fixtures/review-editor-upgrade.sql` → current annotation storage | Retained under migration CI coverage, including preservation and replay. Exact production source must match fixture assumptions |
| `scripts/migrate-unit-flagged.sql` | Legacy Page/Unit schema used by its test, without `f_is_flagged` → BOOLEAN flag column | Legacy only. `test-unit-flagged-upgrade.sh` still uses `t_page` and `f_page_id`, so it does not establish compatibility with current PageSource storage. Do not use as a generic 1.0 upgrade |
| `scripts/migrate-chapter-issue-baseline.sql` | ChapterArtwork with `f_key` → historical confirmation baseline | Unsupported against current `f_domain` schema; also adds a retired business CHECK. Retain until production execution and archival disposition are established |
| `scripts/prom-upgrade/` | Numeric lease state described by the old manual → UUID-token snapshot that required business CHECKs | Historical conversion, not a supported current-baseline upgrade. Its contract/test require CHECKs absent from current migrations. `test-prom-upgrade.sh` is not invoked by migration CI |
| `scripts/migrate-unit-v2-storage.sql` | Indexed Unit storage → historical linked-list storage | Historical conversion without current-baseline CI coverage. Source columns, PageSource identity and applied-version history require reconciliation before any future support claim |
| `docs/image-kind-migration.md` | Historical image-task JSON key `resource_kind` → `image_kind` | Historical note only; does not convert current fixed-topic Prom payloads |
| `tests/fixtures/db_bridge_v1_old.sql` | Old bridge schema and identifiers | Removed after reference review: no invoking test/script or supported rehearsal; not an applied migration source |

Source versions for several legacy conversions were never recorded as semantic
releases. Their structural preconditions and fixtures are the available source
identity; do not invent a version range. A legacy tool becomes supported only
after matching its actual source schema, target baseline, data-preservation,
replay, rejection, and rollback/recovery tests. Running an old script in CI is
not by itself evidence of compatibility with the current baseline.

## Production acceptance

Record the running version, actual schema and migration history, source/target
preconditions, prior conversion results, backup restoration evidence, allowed
rollback image/schema combinations, and owner for unresolved queue work before
the first stable cutover. Where these are unknown, the upgrade remains blocked.
Retire old tools from active directories only after the necessary evidence is
preserved independently of the old Git history. Current production migrations,
data conversions, resets, and deployment operations require their normal
authorization and GitHub Actions controls.
