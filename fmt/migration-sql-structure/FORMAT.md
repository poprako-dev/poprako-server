# Migration SQL structure

Business migrations are immutable baseline definitions. Patch-style migrations
are forbidden.

Every migration directory after the Diesel setup and feature-enablement
migrations must use one of these forms:

| Directory suffix | `up.sql` responsibility | `down.sql` responsibility |
| --- | --- | --- |
| `create-<name>-table` | Create exactly one table and nothing else | Drop that same table |
| `index-<name>-table` | Create one or more indexes for exactly that table | Drop exactly those indexes |
| `seed-<name>` | Insert rows into exactly one table | Delete rows from that same table |

The `<name>` in create and index migration names maps to the database table
`t_<name>`, replacing hyphens with underscores.

## Forbidden patterns

- Unclassified business migration names, including patch, backfill, add-column,
  and fix-up migrations.
- More than one table in a create migration.
- Indexes or seed data inside a create migration.
- Indexes for multiple tables in one index migration.
- Seed data for multiple tables in one seed migration.
- `ALTER`, `DROP`, `UPDATE`, or `DELETE` statements in `up.sql`.
- Asymmetric up/down table or index targets.

## Diagnostics

| Code | Meaning |
| --- | --- |
| `MIG001` | Invalid or unclassified migration layout |
| `MIG002` | Invalid single-table create migration |
| `MIG003` | Invalid single-table index migration |
| `MIG004` | Invalid single-table seed migration |
| `MIG005` | Up/down migration targets are asymmetric |

## Usage

```bash
python3 fmt/migration-sql-structure/check.py
python3 fmt/migration-sql-structure/check.py --self-test
```
