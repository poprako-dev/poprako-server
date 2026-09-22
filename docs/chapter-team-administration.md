# Chapter management through team administration

Chapter management belongs to the owning team's administrators. A team admin
does not need a chapter assignment to edit metadata, pin chapters, manage worker
assignments or invitations, change workflow stages, or upload chapter artwork.
Management privileges follow current team membership; former chapter admins are
not automatically promoted to team admins.

Assignments contain worker roles only. Creating a comic or chapter without
`preset_assignment_roles` creates no assignment. Explicit presets still require
the corresponding team worker roles. ADMIN in assignment writes, invitations,
presets, or assignment-list role filters is rejected as invalid input.

Team admins can advance workflow stages without a worker role holder. Workflow
state transitions and published-chapter freezing still apply. Translation,
proofreading, original-page uploads, and other worker operations retain their
existing assignment requirements.

Translation export remains available to team members and chapter assignees.
Only a TYPESETTER or REDRAWER assignment starts pending typeset/redraw work on
export. Team administration alone, and other worker assignments, do not start it.

## Migration and compatibility

The standalone upgrade deletes admin-only assignments and removes the admin
timestamp column and its two indexes. Mixed assignments retain their IDs, worker
roles, and worker timestamps. Immutable workflow and archive snapshots retain
their historical role values.

Baseline migrations create the target schema directly and contain only their
declared table/index responsibilities. They do not upgrade existing tables.
Existing databases require `scripts/chapter-admin-upgrade/apply.sql` before the
new application deploys. The standalone upgrade is transactional and replayable;
it locks assignments and compares every retained row before committing.

Older application versions reference the removed column. Stop all old business
processes before upgrading and leave them stopped until the new version deploys.
For a pre-merge upgrade, this means an explicit maintenance window spanning PR
merge and deployment. Do not restart the old image against the upgraded schema.
Rollback requires the pre-upgrade backup; restoring a nullable column alone does
not restore deleted assignments or administrator identities. Normal application
deployment remains in GitHub Actions; manual database execution requires explicit
user authorization.

## Existing production database upgrade

Confirm the target is `db_poprako_server_prod`, stop all business processes, and
take a PostgreSQL custom-format backup with `pg_dump --format=custom`. Keep it
outside the database container with restrictive permissions. Verify the archive
with `pg_restore --list`; a tested restore remains the recovery prerequisite.
Do not print connection secrets or use `diesel migration revert --all` on production.

Run from the repository root, with the approved target connection in the environment:

```sh
psql "$DATABASE_URL" -X --set ON_ERROR_STOP=1 --file scripts/chapter-admin-upgrade/apply.sql
psql "$DATABASE_URL" -X --set ON_ERROR_STOP=1 --file scripts/chapter-admin-upgrade/verify.sql
```

When executing through `ssh prk`, copy the `scripts/chapter-admin-upgrade/`
directory together so relative includes resolve. Use the PostgreSQL container's
client and credentials without exposing them. Record the backup path, image
version, upgrade result, and post-upgrade verification before handing back to CI/CD.

Clients should derive management controls from owning-team membership and must
not expect a creator assignment or ADMIN in current assignment responses. Route
paths and request field names remain unchanged.

## Validation

Use `scripts/ci-migration-check.sh` on the authorized `db_poprako_ci` database for
apply, full migration replay, revert-all, and apply verification. It also runs
the legacy data-preservation fixture and the standalone upgrade twice.

The data-preservation regression runs in a rolled-back transaction:

```sh
psql "$CI_DATABASE_URL" --set ON_ERROR_STOP=1 --file tests/fixtures/chapter-admin-migration.sql
```

The fixture refuses any database other than `db_poprako_ci` and checks admin-only
deletion, mixed-role preservation, replay, and test-only reconstruction of the old
column/index structure. Test reconstruction is not a production rollback script.
