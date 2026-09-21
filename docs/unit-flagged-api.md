# Unit review flags

`is_flagged` is a shared Unit boolean for text that an editor wants to revisit.
Assigned translators and proofreaders can set or clear it through the ordinary
Unit save endpoint. It is independent of approval, progress counts, and workflow
stages. Text edits and text clearing preserve it. Delete retains it on the
tombstone; restoring the Unit preserves it unless the patch supplies a value.

## Save and read

`POST /api/v1/pages/{page_id}/units/save?save_id={uuid}` accepts the existing
edit array and returns the existing `200` save receipt:

```json
[{ "edit": "patch", "id": "unit-id", "is_flagged": true }]
```

Create accepts `is_flagged`, defaulting to `false` when omitted. Patch accepts
`true` or `false`; omission and `null` preserve the current value. Explicit
`false` cancels the flag. Repeated patches retain the last explicit value.
The field is not a tagged Clear/Assign patch. Changing only the flag preserves
translator/proofreader attribution and follows normal save timestamp updates.

Unit list and text-search responses always include the boolean. PopRaKo JSON
exports and newly created comic archives retain it. PopRaKo imports default a
missing field to `false`; keep mode retains existing Units unchanged. LabelPlus
has no flag representation, so newly imported LabelPlus Units are unflagged.
Stored historical archives are not rewritten.

Save receipt hashing preserves the serialized bytes of pre-flag requests:
Create's default false and Patch's absent flag are omitted from serialization.
An explicit Patch false participates in the digest. Reusing a save identity
with a different flag patch is rejected like any other changed payload.

## Page navigation

`GET /api/v1/chapters/{chapter_id}/pages/unit-flagged-stats` returns `200` with
the normal response envelope. Its `data` is an array:

```json
[{ "page_id": "page-id", "index": 2, "flagged_unit_count": 3 }]
```

Only visible flagged Units count. Pages with zero matches are omitted; an empty
result is `[]`. Original zero-based Page indexes are preserved, ordered by
`index`, then `page_id`. The frontend opens the Page and uses its Unit list to
highlight flags. Access, missing-Chapter errors, and the complete Chapter page
limit follow `unit-diff-stats`. Statistics are calculated on demand.

## Existing database upgrade

New databases receive `f_is_flagged BOOLEAN NOT NULL DEFAULT FALSE` from the
single-table Unit baseline. Replaying `CREATE TABLE IF NOT EXISTS` does not
upgrade an existing table.

Run the standalone upgrade explicitly against the intended existing database
before starting the new server:

```sh
psql "$DATABASE_URL" -X -v ON_ERROR_STOP=1 -f scripts/migrate-unit-flagged.sql
```

This script is not wired into Actions or application startup. It takes a table
lock and adds the column in one transaction. Existing rows receive false without
changing text, order, visibility, attribution, or timestamps. Repeated runs
validate the boolean type, non-null constraint, and false default, preserving
existing flags. An incompatible existing column aborts the transaction.
Rolling back the application can leave this additive column and its data intact.

After preparing the development database, regenerate schema with
`just mgr-schema`; never edit the generated schema by hand.

The disposable database upgrade regression can be run after baseline migrations:

```sh
CI_MIGRATION_DATABASE=1 DATABASE_URL="$CI_DATABASE_URL" \
    sh scripts/test-unit-flagged-upgrade.sh
```

`CI_DATABASE_URL` must target `db_poprako_ci`. The test exercises old visible and
hidden rows, preservation of data and timestamps, schema agreement with the
baseline, repeated execution after flagging, and rejection of wrong type,
nullability, or default. It intentionally changes the disposable schema.
