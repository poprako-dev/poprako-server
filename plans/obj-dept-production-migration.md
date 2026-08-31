# ObjDept Production Migration Plan

## Purpose

This plan prepares the `feat/obj-dept` changes for a safe production cutover.
It covers the relational schema migration, legacy R2 object movement,
application downtime, verification, rollback, and the final GitHub Actions
deployment.

All production writes, container lifecycle changes, migrations, backups, and
deployments must run from the protected GitHub Actions `production`
environment through the dedicated deployment account. Maintainer machines may
use `ssh prk` only for read-only inspection.

This document records the production state observed on 2026-08-30. Counts must
be collected again after the application is stopped because production data is
still changing.

## Current production state

### Runtime

- Host: `PopRaKo-VPS`, with approximately 2 GiB RAM and 17 GiB free root disk.
- Application container: `poprako-server-prod`, healthy, running commit
  `ae5e1cba12d5b0ae0ff7bb0a5f0c1ecbede5fd9e`.
- Previous application container:
  `poprako-server-prod-previous`, stopped.
- PostgreSQL container: `poprako-postgres-prod`, PostgreSQL 18.4.
- Docker network: `poprako-prod`.
- Database: `db_poprako_server_prod`, approximately 11 MiB.
- PostgreSQL data is bind-mounted from `/root/postgres-database/data`.
- `pg_dump` 18.4 is available inside the PostgreSQL container.
- The application reported no `ERROR`, panic, or fatal runtime log in the
  inspected 24-hour window.

The deployed application is behind `main`: commit `7bb27710...` reached
`main`, but its Production image job failed, so it was not deployed. The
failure was caused by the Dockerfile not copying the path workspace member
`poprako-obj-dept`. The current branch adds more path crates and still requires
the corresponding Dockerfile fix.

### Database migration mechanism

Production has no `__diesel_schema_migrations` table. The deployment script
currently concatenates every checked-in `up.sql` file into one transaction on
every deployment.

Consequences:

- editing an old `CREATE TABLE IF NOT EXISTS` migration does not alter the
  already-existing production table;
- removing legacy columns from old create-table files only fixes fresh
  databases;
- every new production migration must tolerate being replayed;
- the current deployment order, migration before stopping the old
  application, is unsafe for this schema change.

### Legacy object metadata

The production database contained the following metadata at inspection time:

| Object kind | Business rows | Complete metadata | Uploaded | Partial or invalid |
|---|---:|---:|---:|---:|
| Page image | 422 | 422 | 422 | 0 |
| User avatar | 33 | 5 | 5 | 0 |
| Team avatar | 4 | 2 | 2 | 0 |
| Comic cover | 10 | 2 | 2 | 0 |

There were 431 uploaded objects in total. All observed versions were `1`, all
hashes were 32 bytes, all extensions were supported, and all stored keys
matched the legacy key grammar. `t_user.f_avatar_source` had no non-null data.

Production remained active during inspection. In the preceding 24 hours,
59 pages, 5 users, and 1 comic had been updated. These counts therefore are
evidence, not the final migration manifest.

### Legacy deferred messages

The legacy `t_local_message` queue contained 200 completed `image` messages and
no pending, processing, or dead `image` message. One unrelated, not-yet-visible
invitation purge task remained pending.

The cutover must repeat this check after shutdown. It must not proceed while an
unfinished legacy `image` task or an unverified object reservation exists.

## Key migration

The new ObjDept key cannot be derived by merely reusing the stored legacy key.
The physical R2 object must be copied.

| Kind | Legacy physical key | New physical key |
|---|---|---|
| Page image | `page/chapter_{chapter_id}/{id}-{version}.{ext}` | `page_image/{base64url(id)}/{version}` |
| User avatar | `user_avatar/{id}-{version}.{ext}` | `user_avatar/{base64url(id)}/{version}` |
| Team avatar | `team_avatar/{id}-{version}.{ext}` | `team_avatar/{base64url(id)}/{version}` |
| Comic cover | `comic_cover/{id}-{version}.{ext}` | `comic_cover/{base64url(id)}/{version}` |

The new key omits the extension. Copying must preserve object metadata,
especially `Content-Type`.

The full production R2 source/destination inventory has not yet been verified.
That check is a hard precondition for cutover.

## Required implementation

### 1. Gate production deployment

Before merging the ObjDept branch:

1. Change production deployment from automatic `push` deployment to an
   explicit `workflow_dispatch` using an immutable commit SHA and image digest.
2. Configure a required reviewer on the GitHub `production` environment. It
   currently has only a `main` branch policy.
3. Keep production image construction in CI, but do not let a merge
   automatically connect to production.
4. Make the production workflow reject a SHA that is not reachable from
   `main`, lacks successful required checks, or does not match the selected
   GHCR digest.

### 2. Repair the production image build

Update the Dockerfile to copy every path workspace member required by the stub
and final build, including:

- `poprako-obj-dept`;
- `poprako-obj-dept-macro`;
- `poprako-rdb-core`;
- all other path members listed in the workspace manifest.

Add a checked-in test that fails when a path workspace member is absent from
the Docker build context. Build the production image during PR validation so a
failure cannot first appear after merging to `main`.

### 3. Correct migration history

Restore the removed legacy object columns in the four historical create-table
migrations. Historical migrations should continue to describe the schema that
they originally introduced.

Keep the new migrations that create:

- `t_page_image`;
- `t_user_avatar`;
- `t_team_avatar`;
- `t_comic_cover`;
- `t_obj_prom_task`;
- the two `t_obj_prom_task` indexes.

Add a later, dedicated legacy-object cutover migration. Its `up.sql` must:

1. Detect whether the legacy columns still exist and safely skip the legacy
   transformation when they are already absent.
2. Reject partial metadata tuples.
3. Reject versions outside the `u32` range, non-32-byte hashes, unsupported
   extensions, or keys that do not match the legacy grammar.
4. Insert only complete metadata into the matching object table.
5. Preserve the business identifier and version and copy the uploaded flag,
   hash, extension, and deterministic source timestamps.
6. Reject conflicting target rows instead of overwriting them.
7. Verify source and target counts before dropping anything.
8. Drop all legacy object columns, including `t_user.f_avatar_source`, only
   after the verification succeeds.

The SQL must run in the deployment's outer transaction and must be replay-safe.
Use conditional dynamic SQL for statements that refer to columns removed by a
previous successful run.

The matching `down.sql` must recreate the legacy columns and reconstruct their
metadata and legacy key values. It exists for disposable-CI migration
validation and rehearsed rollback; production should normally restore its
pre-cutover dump.

After editing migrations, regenerate Diesel `schema.rs` from the resulting
schema and run the full apply -> revert-all -> apply cycle against
`db_poprako_ci`.

### 4. Implement an R2 migration utility

Add a dedicated utility used only by the protected maintenance workflow. It
must reuse the production ObjDept key encoder and support these modes:

- `check`: build the source/destination manifest, require every uploaded source
  object to exist, and detect destination collisions;
- `copy`: copy objects within the same R2 bucket without deleting the source;
- `verify`: compare object count, length, content type, and SHA-256 with the
  database metadata;
- `reverse`: reconstruct legacy keys for a rehearsed rollback.

Operational requirements:

- idempotently skip an existing destination only when it is verified equal;
- fail on an existing destination with different content or metadata;
- preserve `Content-Type` and other required R2 metadata;
- stream verification without retaining complete objects on disk;
- use bounded concurrency of approximately two to four operations for the
  low-resource VPS;
- never print object keys, identifiers, URLs, or credentials to Actions logs;
- output only aggregate counts and failure categories;
- never delete legacy objects during the initial cutover.

The manifest may contain sensitive identifiers. Keep it only in the protected
backup area, checksum it, and do not publish it as an unencrypted Actions
artifact.

### 5. Implement a protected maintenance workflow

The maintenance workflow must use the repository's checked-in POSIX `sh`
scripts directly. It must not depend on `just`, a maintainer machine, or an
interactive production SSH session.

Required workflow inputs include the exact deployment SHA and an explicit
cutover confirmation. The workflow should have a sufficiently long timeout for
R2 verification and must serialize with every other production deployment.

Add failure-injection tests for at least:

- image pull failure before downtime;
- backup failure;
- R2 source missing;
- R2 destination conflict;
- database validation failure;
- migration failure;
- new container health failure;
- post-deployment smoke-test failure;
- database restore and old-container recovery.

## Cutover runbook

### Phase A: before the maintenance window

1. Freeze and review the branch. Preserve unrelated working-tree changes.
2. Require every PR check to pass, including:
   - Rust checks and tests;
   - migration apply -> revert-all -> apply;
   - deployment-script tests;
   - production image build;
   - object-migration utility tests.
3. Merge only after automatic production deployment is disabled and the
   `production` reviewer gate is active.
4. Let CI build and publish the production image for the merge SHA.
5. From the protected workflow, run the R2 utility in `check` mode.
6. Optionally perform a non-destructive online pre-copy to reduce downtime.
   The application remains authoritative until it is stopped, so this is not
   the final manifest.
7. Announce the maintenance window and block unrelated production deployment
   runs.

Stop the cutover if the source inventory is incomplete, a destination
collision exists, CI is not green, or the target image cannot be pulled and
verified.

### Phase B: enter maintenance

1. Pull and digest-verify the new image before stopping the application.
2. Stop `poprako-server-prod` through the protected workflow.
3. Confirm that it is stopped and that PostgreSQL has no remaining application
   write connection.
4. Generate a fresh database/object manifest.
5. Require:
   - no partial legacy metadata;
   - no `uploaded = false` reservation;
   - no unfinished legacy `image` message;
   - every uploaded legacy object to be readable;
   - no conflicting new-key object.
6. Copy and verify every remaining R2 object. Do not remove the legacy copy.

If any check fails, restart the unchanged old application and leave the
database schema untouched.

### Phase C: backup

1. Create a PostgreSQL custom-format dump with `pg_dump -Fc`.
2. Create a separate schema-only dump.
3. Store both under a dedicated backup directory outside release-directory
   cleanup.
4. Record SHA-256 checksums.
5. Run `pg_restore --list` against the custom-format dump.
6. Store and checksum the final R2 migration manifest beside the dumps.

The database is only about 11 MiB, so a complete logical backup is preferred
over a selective backup.

Do not continue if backup creation or validation fails.

### Phase D: migrate the database

Apply the checked-in migrations in a single transaction. Before commit, verify
inside the same transaction that:

- all four object metadata tables contain the expected rows;
- every migrated active tuple is internally valid;
- the target counts equal the stopped-production manifest counts;
- `t_obj_prom_task` and both indexes exist;
- all expected non-object tables, columns, constraints, and indexes remain;
- all legacy object columns are absent;
- no unexpected row was added to `t_obj_prom_task`.

At the 2026-08-30 observation point, the expected active counts would have been
422 page images, 5 user avatars, 2 team avatars, and 2 comic covers. The
cutover must use freshly collected counts instead.

### Phase E: deploy and verify

1. Start the new application container from the exact approved image digest.
2. Wait for the container health check.
3. Reject startup logs containing `ERROR`, panic, or fatal runtime errors.
4. Verify detailed metrics are available.
5. Run authenticated smoke tests for:
   - page list and detail;
   - user list/detail and avatar URL;
   - team list/detail and avatar URL;
   - comic list/detail and cover/fallback URL;
   - representative list paths that now perform batched metadata reads.
6. Verify every migrated uploaded object through its new URL or, at minimum,
   perform full R2 verification plus representative HTTP/CDN checks.
7. Inspect `t_obj_prom_task` for unexpected retry, stuck, or operator-repair
   rows.
8. Keep the service in the maintenance acceptance window until all checks pass.

Only after these checks pass may normal traffic and writes resume.

## Rollback

### Before database commit

The migration transaction rolls back. Restart the unchanged old application.
Copied R2 objects are harmless because the old application does not reference
the new keys.

### After database commit but before reopening writes

The current automatic `restore_previous` behavior is unsafe because the old
binary expects columns that the new schema removed.

The workflow must instead:

1. stop and isolate the failed new container;
2. terminate its database sessions;
3. restore the validated pre-cutover PostgreSQL dump;
4. verify the restored legacy schema and row counts;
5. restart the previous container;
6. run old-version health and API checks.

The legacy R2 objects remain in place, so the restored old application retains
valid object URLs.

### After reopening writes

Do not restore the pre-cutover dump after accepting new writes; doing so would
lose post-cutover data. From that point onward, use a forward fix or an
explicitly reviewed reverse metadata/R2 migration that preserves new writes.

Retain the database backup, previous image/container, final manifest, and
legacy R2 objects for an observation window of at least 7 to 14 days. Delete
legacy objects only through a later, separately approved GitHub Actions cleanup
workflow after a complete inventory comparison.

## Final CI/CD sequence

1. Complete the deployment gate, Dockerfile, migration, R2 utility, workflow,
   and rollback implementation.
2. Pass all PR checks.
3. Activate the GitHub production reviewer gate.
4. Merge the PR without automatically deploying it.
5. Build and publish the merge-SHA image through CI.
6. Trigger and approve the protected ObjDept maintenance workflow.
7. Complete stop, final inventory, backup, R2 copy, database migration, new
   image deployment, and verification in that workflow.
8. Monitor application logs, metrics, database connections, and
   `t_obj_prom_task` during the observation window.
9. Create a tag or GitHub Release later only if desired. A release is not a
   prerequisite for this production deployment, and any release must use the
   established GitHub Actions Release workflow.

## Acceptance criteria

The migration is complete only when all of the following hold:

- production runs the approved merge SHA by immutable image digest;
- the live schema equals a fresh application of the checked-in migrations;
- every legacy metadata tuple was migrated exactly once;
- every uploaded object is readable through the new ObjDept key;
- legacy business-table object columns are absent;
- no unfinished legacy image message was abandoned;
- health, logs, metrics, and representative authenticated API requests pass;
- the pre-cutover database dump and legacy R2 objects remain available for the
  rollback window;
- all production mutations and deployment steps have an auditable GitHub
  Actions run.
