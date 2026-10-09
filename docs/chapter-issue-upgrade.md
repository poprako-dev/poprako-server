# Chapter issue confirmation baseline upgrade

Fresh databases receive `f_confirmed_artwork_version` from the single-table
Chapter baseline and `t_issue` from its own single-table migration. Replaying
`CREATE TABLE IF NOT EXISTS` does not add a column to an existing Chapter table.

Existing databases require the explicit standalone upgrade before the new
server starts. Stop business processes and take a restorable backup, then run
against the intended database with explicit authorization:

```sh
psql "$DATABASE_URL" -X -v ON_ERROR_STOP=1 -f scripts/migrate-chapter-issue-baseline.sql
```

The script runs in one transaction, locks Chapters and artwork, adds the
nullable BIGINT confirmation column with the unsigned 32-bit range constraint,
and backfills Chapters whose current artwork is uploaded and has a key, hash,
and extension. It preserves recorded confirmations, other Chapter fields,
timestamps, and artwork rows. An incompatible existing column aborts the
transaction. Pending or incomplete artwork leaves the confirmation baseline null.

Repetition before the new server starts preserves recorded baselines. Do not
use this script during ordinary operation: backfilling a null baseline after a
new upload would treat that artwork as already confirmed. Apply the normal
baseline migrations through GitHub Actions to create `t_issue`; application
startup never runs either upgrade or migrations. The script is not wired into
Actions. Rolling back the application can leave the additive column intact.

After preparing the development database, regenerate schema with
`just mgr-schema`; never edit generated schema by hand.
