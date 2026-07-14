# HTTP Integration Test Inventory

This is the maintenance inventory for the TypeScript API suite. Whenever a file
under `src/suites/` is added, removed, renamed, or materially changes scope,
update this document in the same change. The executable suite and its assertions
are the source of truth; do not keep separate implementation plans or status
trackers here.

## Running the suite

```text
cd tests/integration-tests
pnpm typecheck
```

For an isolated end-to-end run, configure `INTEGRATION_DATABASE_URL` and run:

```text
scripts/api-integration-test.sh
```

The script creates and drops the dedicated integration database. Do not point
it at a database that contains data you need to preserve.

## Suite order

`src/main.ts` resets the database, runs these modules in order, then restores
the seed-only state in its `finally` block. Every current module exports
`IMPLEMENTED = true`.

| Module | File | Coverage |
| --- | --- | --- |
| it_00 | `it_00_bootstrap_auth_default_seed.ts` | Seed data, login, and unauthenticated access. |
| it_01 | `it_01_member_invitation_register_roles.ts` | Invitations, registration, member lists, and role permissions. |
| it_02 | `it_02_workset_comic_chapter_index.ts` | Workset, comic, chapter indexes, pinning, profile updates, and positionally aligned comic/pinned-chapter list payloads. |
| it_03 | `it_03_page_reserve_image.ts` | Page reservation, image confirmation, and page rebuilds. |
| it_04 | `it_04_assignment_invitation.ts` | Assignment joins, invitations, updates, and deletion. |
| it_05 | `it_05_unit_save_order_count.ts` | Unit ordering, counts, and translation import/export. |
| it_06 | `it_06_unit_concurrency.ts` | Parallel unit writes, merge behavior, and replay. |
| it_07 | `it_07_workflow_sysmail.ts` | Workflow transitions and deferred system mail. |
| it_08 | `it_08_info_update_upload_mark.ts` | Resource updates, uploads, announcements, comments, and profiles. |
| it_09 | `it_09_cross_team_permission.ts` | Cross-team authorization isolation. |
| it_10 | `it_10_cascade_delete_cleanup.ts` | Cascade deletion and cleanup side effects. |
| it_11 | `it_11_comic_archive.ts` | Immutable comic archive rows and image-delete prom records. |

## Shared fixtures and invariants

- `src/db/seed.ts` owns reset, cleanup, and seed-only assertions.
- `src/http/fixtures.ts` owns reusable API operations.
- `src/http/invariants.ts` owns counter, index, workflow, export, and mail
  consistency assertions.
- `src/state/runCtx.ts` is the shared state passed between modules.

When a test reveals a contract change, update the tested source, the relevant
module assertion, and any affected active API document together.
