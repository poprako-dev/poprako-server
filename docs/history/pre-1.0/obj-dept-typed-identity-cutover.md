# ObjDept typed identity refactor

Status: strict persistence implemented and production repair completed.

Retained operational record: preserve the dated production result and private
backup/report location below independently of old Git history. The repair
authorization described here applied to that completed operation; it does not
authorize a new production action. Current contracts live in
[the ObjDept specification](../../../specs/obj-dept.md); current upgrade support is
defined in [the database support matrix](../../database-upgrade-support.md).

## Required contract

Use a fresh database as the baseline. The server supports one current format
and contains no historical path parser, fallback, alias, or arbitrary-path
restoration. Typed object identities remain typed through allocation, metadata,
operations, task creation, and static dispatch.

Object rows and tasks persist complete construction data in `f_domain JSONB`.
Each object kind owns a strict serializable domain type. Raw persistence is
converted once into that type, validating IDs, parent IDs, extensions, and
agreement with independent owner/suffix columns. Physical paths are generated
by the current mapping. Missing or extra fields fail restoration.

Ordinary replacement and deletion retain enough construction data to clean up
retired generations after business rows disappear. Revision CAS, version
watermarks, claim fencing, transaction ownership, and retry semantics remain
part of normal operation on a new database.

## Implementation

1. Replace raw path snapshots with domain JSON in records, generated Diesel
   storage, and task envelopes. Remove the physical-path database column.
2. Validate serde restoration of every business ID and extension, reject
   unknown domain fields, and compare restored owner/suffix with row columns.
3. Replace historical-format acceptance tests with rejection tests. Exercise
   ordinary cleanup and concurrency using current canonical construction data.
4. Update the seven baseline table definitions and regenerate schema only via
   `just mgr-schema`. Validate apply, replay, revert-all, and apply on the
   authorized disposable `db_poprako_ci` database.
5. Correct the active spec, ownership audit, and identity skill. Do not retain
   any requirement that the new persistence representation accept old data.
6. Complete canonical formatting, workspace check, Clippy, affected database
   tests, and the workspace test workflow.

## Explicit production repair

The user authorized repair through `ssh prk`. The target is
`db_poprako_server_prod` in `poprako-postgres-prod`. The API is already stopped.
Production data stays on the server; reports contain aggregate counts.

Create a server-local backup and rehearse against a new server-local clone.
Keep the offline repair separate from the service. Business data reads and
writes use typed Diesel expressions.
Retain a private reversible ID map, convert primary and reference IDs together,
update exact references inside workflow payloads, and preserve business state.
Clear all user and team avatar metadata while preserving version watermarks.
Delete all corresponding R2 objects without retaining image originals, as
explicitly requested. The audited non-avatar object rows are all detached.
Finalize the schema by removing retired path columns
and restoring the original foreign-key deferral policy. Verify canonical IDs,
referential integrity, object metadata, and zero remaining avatars before reporting
completion. No server release or deployment is implied by this repair.

## Production result

On 2026-10-25, the offline repair normalized 10,706 business identities and their
references without changing business row counts. All nine user-avatar and two
team-avatar watermarks are detached, retaining only their version state. The
configured R2 bucket returned `NoSuchBucket`; the user confirmed its deletion
and the absence of another avatar store. No avatar image bytes were retained.
Six workflow records now use `imported_page_source_count`. The retired issue
layer field was removed in coordination with the user's parallel changes.
All 29 business tables match the generated schema's column sets, all 33 foreign
keys remain present, and the original single deferred constraint is preserved.
The API remains stopped. The private backup, identity map, audit and completion
report remain under `/opt/poprako-server/maintenance/obj-identity-20261025`.
