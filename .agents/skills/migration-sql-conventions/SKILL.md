---
name: migration-sql-conventions
description: Create, change, or review PopRaKo migration SQL, including aligned layout, business-validation boundaries, paired up/down definitions, and Diesel rollback or reset verification.
---

# Migration SQL conventions

This skill owns the project's migration SQL conventions. Paths and commands
below are repository-relative. Follow [root AGENTS.md](../../../AGENTS.md)
for database authorization, generated files, production operations, and Rust
validation; applying this skill does not authorize a database reset.

## SQL layout

- Quote table, column, and index identifiers consistently with double quotes.
  Use uppercase SQL keywords and types, four-space indentation, and one
  field or seed value per line.
- Align field names, types, and following declarations into columns within
  each table. Size padding to the longest name and type in that table;
  omit trailing padding when no declaration follows the type.
- Separate field groups with one blank line according to their meaning:
  identity; parent identity and position; related business data; counters;
  status or lifecycle; creation and update timestamps. Keep tightly related
  fields together. Do not split every field into its own group.
- Keep a standalone entity ID in its own first group. Keep `created_at` and
  `updated_at` together. Preserve established field order for focused edits;
  when rewriting an entire baseline, place the audit timestamps last.
- Separate independent SQL statements with one blank line in both directions.
  Seed column and value groups must correspond. Keep index expressions on
  one line when readable; use the same wrapping style for comparable indexes.

Use [the Workset baseline](../../../migrations/2026-07-17-083504-0000_create-workset-table/up.sql)
as a compact alignment and grouping example. Choose groups for the actual
table rather than copying that example's business fields.

## Business validation belongs in application code

- Primary keys, storage nullability (`NOT NULL`), foreign keys, unique indexes,
  and ordinary defaults are allowed structural definitions.
- Never add business `CHECK` constraints or explicit business `CONSTRAINT`
  clauses. This includes numeric ranges, nonblank or formatted text, allowed
  status values, JSON shape, geometry, optional-field combinations, and
  state/token consistency.
- Do not move those checks into triggers, database functions, procedures,
  domains, or another database mechanism as a workaround. Keep business rules
  in the domain/application layer and persistence concerned with storage.
- Removing a database business check must retain the corresponding business
  validation. Use [test-spec](../test-spec/SKILL.md) when adjusting tests:
  invalid business input is rejected at the application boundary; RDB tests
  cover persistence, foreign keys, uniqueness, and transaction rollback.

## Paired baseline definitions

Business migrations under `migrations/` use one responsibility per directory:

| Directory suffix | `up.sql` | `down.sql` |
| --- | --- | --- |
| `create-<name>-table` | Create exactly one table | Drop that table |
| `index-<name>-table` | Create indexes for exactly one table | Drop exactly those indexes |
| `seed-<name>` | Insert into exactly one table | Delete the corresponding seed rows |

- Keep secondary unique indexes in the table's index migration, with matching
  drops. Do not hide indexes or seeds inside a create-table migration.
- Keep baseline definitions directly in the affected table/index/seed files;
  do not introduce patch-style migrations to evade the baseline structure.
- Use `IF NOT EXISTS` and appropriate seed conflict handling so forward files
  support the project's production replay. Rollback files use `IF EXISTS`
  where applicable and respect reverse dependency order.
- Never remove or rename an applied migration version before its authorized
  rollback. Before rewriting an applied baseline, reconcile the intended
  target's Diesel history with the available migration sources.

## Diesel reset and missing-history recovery

- A requested reset means `just mgr-reset` (`diesel migration revert -a`).
  Preserve the database itself. Do not substitute database deletion or
  recreation, `diesel database reset/drop`, `TRUNCATE`, manually deleting
  migration records, or direct schema destruction outside Diesel.
- A failed full rollback is a P0 migration defect. Diagnose the exact failure
  rather than reporting cleanup as complete. If a recorded version is missing,
  recover its genuine rollback source from history; when unavailable,
  reconstruct only verified objects and dependencies for that version.
- Run the recovered rollback through Diesel. Never use an empty/fake
  `down.sql`, blanket `CASCADE`, or a fabricated successful migration history
  to bypass the failure. Retire temporary recovery sources only after Diesel
  has successfully reverted their recorded versions.
- Verify that rollback leaves zero business tables and zero migration history
  entries. Diesel's bookkeeping table may remain. Preserve the requested final
  state: a reset alone does not imply permission to repopulate the target.
- An approval rejection is not a SQL failure. Respect it, report the blocked
  action and reason, and obtain the missing target confirmation or use the
  already-authorized CI database for independent validation.

## Validation

- Run `sh scripts/ci-migration-check.sh` against the authorized `db_poprako_ci`
  target with `CI_MIGRATION_DATABASE=1` and its `DATABASE_URL`. The checked-in
  script validates apply, repeated forward replay, revert-all, an empty
  business schema/history, and apply again. Never replace this with deleting
  the CI database.
- For schema changes, regenerate only with `just mgr-schema` against the
  intended migrated database; never hand-edit `schema.rs`. Diesel migration
  commands may also regenerate it through `diesel.toml`, so inspect the final
  generated file after a reset and restore it through the permitted recipe
  when necessary.
- Review field types, nullability, defaults, foreign keys, and paired index/seed
  targets for accidental changes. When business checks are removed, verify
  the migrated database has no remaining business `CHECK` constraints and
  run the affected application and persistence tests.
