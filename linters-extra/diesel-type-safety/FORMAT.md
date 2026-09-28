# Typed Diesel queries only

All workspace Rust, including tests, mocks, build scripts, examples, benches,
generated schema and macro token trees, must use Diesel's typed query DSL.
There are no per-file exemptions or inline suppression attributes.

`SQL001` rejects reserved escape-hatch identifiers: `sql`, `sql_query`,
`SqlLiteral`, `SqlQuery`, `BoxedSqlQuery`, `UncheckedBind`, `QueryableByName`,
`NamedRow`, `SimpleConnection`, `AsyncSimpleConnection`, `batch_execute`,
`simple_query`, `batch_execute_returning_count`, `begin_transaction_sql`,
`push_sql`, `walk_ast`, `get_raw_connection`, and `as_raw_connection`.
Names are forbidden at import/definition/reference sites, regardless of namespace;
this intentionally also reserves local names. Aliasing or re-exporting an entry
point cannot hide its original name. Raw identifiers and macro bodies are checked.
Comments and string/character literals are not executable API references.

`SQL002` rejects known alternative SQL drivers in Cargo dependencies,
including renamed packages and target/workspace/dev/build dependencies. This
prevents replacing Diesel with a string-execution driver to evade SQL001.
The maintained driver set lives in `ALTERNATE_DRIVERS` in `check.py`.
`SQL003` reports invalid manifests or an invalid scan root.

Typed Diesel filtering, joins, subqueries, projections, writes, typed SQL function
declarations, and QueryFragment bounds remain allowed. Implementing custom SQL
rendering via `walk_ast`/`push_sql` is forbidden. SQL migration files and Diesel's
migration harness remain supported; Rust query code cannot use raw execution.

The checker recursively scans the requested root, including nested crates and
custom Rust target locations. It excludes dependency/tool/cache/output directories
listed in `IGNORED_DIRS`; it does not use the production-only source masker.
The existing extra-linter runner discovers this checker and runs its self-tests.

This is a conservative source-level policy gate, not Rust name resolution or a
SQL type checker. It does not expand external macros, evaluate dynamically
generated Rust strings, or prove that an unknown external wrapper is safe.
New driver APIs and generated-code mechanisms still require review; passing this
gate does not authorize bypassing the repository's typed SQL requirement.

```sh
linters-extra/.venv/bin/python linters-extra/diesel-type-safety/check.py --self-test
linters-extra/.venv/bin/python linters-extra/diesel-type-safety/check.py --root .
```
