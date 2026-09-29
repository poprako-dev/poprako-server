"""Regression coverage for the actual forbidden APIs and their hiding places."""

from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from check import check_root, manifest_diagnostics, rust_diagnostics


class TypedDieselTests(unittest.TestCase):
    def test_forbidden_forms(self):
        cases = [
            'fn f() { diesel::sql_query("SELECT 1").bind::<Integer, _>(1); }',
            'use diesel::sql_query as safe_query;',
            'pub use diesel::{dsl::{sql as typed}, sql_query as run};',
            'use diesel as d; fn f() { d::sql_query("SELECT 1"); }',
            'extern crate diesel as d; fn f() { d::dsl::sql::<Text>("x"); }',
            'use diesel::dsl::*; fn f() { sql::<Integer>("1"); }',
            'fn f() { let execute = diesel::sql_query; execute("SELECT 1"); }',
            'fn f() { ::diesel::dsl::r#sql::<Text>("x"); }',
            'fn f() { diesel /* comment */ :: sql_query("SELECT 1"); }',
            '#[derive(diesel::QueryableByName)] struct Row { id: i32 }',
            'type Q = diesel::query_builder::SqlQuery;',
            'type Q = diesel::expression::SqlLiteral<Integer>;',
            'use diesel::query_builder::{BoxedSqlQuery, UncheckedBind};',
            'use diesel::connection::SimpleConnection as Execute;',
            'fn f() { conn.batch_execute("DELETE FROM t"); }',
            'fn f() { conn.simple_query("SELECT 1"); }',
            'fn f() { AnsiTransactionManager::begin_transaction_sql(conn, "BEGIN"); }',
            'fn f() { builder.push_sql("SELECT 1"); }',
            'impl<T> Alias<T> for Query { fn walk_ast(&self, pass: Pass) {} }',
            '#[cfg(test)] mod tests { fn f() { diesel::sql_query("SELECT 1"); } }',
            '#[cfg(any())] fn disabled() { diesel::sql_query("SELECT 1"); }',
            'macro_rules! raw { () => { diesel::sql_query("SELECT 1") }; }',
            'fn generate() { quote! { use diesel::sql_query as run; run("SELECT 1"); }; }',
            'fn generate() { quote! { #driver::sql_query(#statement) }; }',
            '#[allow(SQL001)] fn f() { diesel::sql_query("SELECT 1"); }',
        ]
        for source in cases:
            with self.subTest(source=source):
                self.assertTrue(rust_diagnostics(source.encode(), "fixture.rs"))

    def test_typed_diesel_and_non_code_are_allowed(self):
        source = '''
            // diesel::sql_query("SELECT 1");
            /* nested /* sql */ QueryableByName */
            const DOCUMENTATION: &str = r#"sql sql_query batch_execute tokio_postgres"#;
            fn f() {
                users::table.filter(users::id.eq(1)).select(users::name).load(conn);
                diesel::insert_into(users::table).values(&row).execute(conn);
                diesel::update(users::table).set(users::name.eq("x")).execute(conn);
                diesel::delete(users::table).execute(conn);
                diesel::debug_query::<Pg, _>(&query);
                conn.run_pending_migrations(MIGRATIONS);
            }
            diesel::define_sql_function! { fn gen_random_uuid() -> diesel::sql_types::Uuid; }
            fn typed<T: diesel::query_builder::QueryFragment<Pg>>() {}
            quote! { #table.filter(#table::id.eq(#id)) }
            use testcontainers_modules::postgres::Postgres;
        '''
        self.assertEqual([], rust_diagnostics(source.encode(), "fixture.rs"))

    def test_exact_diagnostic_location(self):
        findings = rust_diagnostics(b'// sql_query\nuse diesel::sql_query;\n', "src/lib.rs")
        self.assertEqual(1, len(findings))
        self.assertTrue(findings[0].startswith("src/lib.rs:2:13: SQL001:"))

    def test_driver_dependency_aliases_and_sections(self):
        for section in ["dependencies", "dev-dependencies", "build-dependencies",
                        "workspace.dependencies", "target.'cfg(unix)'.dependencies"]:
            with self.subTest(section=section):
                source = f'[{section}]\ndb = {{ package = "tokio-postgres", version = "0.7" }}\n'
                self.assertTrue(manifest_diagnostics(source, "Cargo.toml"))
        self.assertTrue(manifest_diagnostics('[dependencies.sqlx]\nversion = "0.8"', "Cargo.toml"))
        self.assertEqual([], manifest_diagnostics(
            '[dependencies]\ndiesel = "2"\ndiesel-async = "0.9"\ndiesel_migrations = "2"', "Cargo.toml"))

    def test_all_source_locations_and_cli_exit(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            locations = ["src/lib.rs", "src/tests.rs", "src/mock_impl.rs", "tests/test.rs",
                         "examples/example.rs", "benches/bench.rs", "build.rs",
                         "nested/member/src/lib.rs", "src/schema.rs", "custom/target.rs"]
            for location in locations:
                path = root / location
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('fn f() { diesel::sql_query("SELECT 1"); }')
            self.assertEqual(len(locations), len(check_root(root)))
            command = [sys.executable, str(Path(__file__).with_name("check.py")), "--root", str(root)]
            failed = subprocess.run(command, capture_output=True, text=True)
            self.assertEqual(1, failed.returncode)
            self.assertIn("SQL001", failed.stderr)
            for location in locations:
                (root / location).write_text('fn f() { diesel::delete(users::table).execute(conn); }')
            self.assertEqual(0, subprocess.run(command, capture_output=True).returncode)

    def test_invalid_manifest_and_missing_root_fail(self):
        self.assertTrue(manifest_diagnostics("invalid = [", "Cargo.toml"))
        with tempfile.TemporaryDirectory() as directory:
            self.assertTrue(check_root(Path(directory) / "missing"))


if __name__ == "__main__":
    unittest.main()
