#!/usr/bin/env -S uv run --script
# /// script
# dependencies = ["tree-sitter==0.25.0", "tree-sitter-rust==0.23.3"]
# ///
"""Reject SQL type-checking escape hatches, including in tests and macros."""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import sys
import tomllib
import unittest

import tree_sitter
import tree_sitter_rust


ROOT = Path(__file__).resolve().parents[2]
PARSER = tree_sitter.Parser(tree_sitter.Language(tree_sitter_rust.language()))
# Reserve these names everywhere: rejecting the original import also prevents
# aliases, re-exports, function pointers, and glob imports from hiding an API.
FORBIDDEN = {
    "sql", "sql_query", "SqlLiteral", "SqlQuery", "BoxedSqlQuery",
    "UncheckedBind", "QueryableByName", "NamedRow",
    "batch_execute", "simple_query", "batch_execute_returning_count",
    "begin_transaction_sql", "SimpleConnection", "AsyncSimpleConnection",
    "push_sql", "walk_ast", "get_raw_connection", "as_raw_connection",
}
ALTERNATE_DRIVERS = {
    "sqlx", "sqlx-core", "sqlx-postgres", "sqlx-mysql", "sqlx-sqlite",
    "postgres", "tokio-postgres", "postgres-native-tls", "postgres-openssl",
    "postgres-protocol", "postgres-types", "pq-sys", "libpq", "libpq-sys",
    "mysql", "mysql_async", "mysql-common", "mysqlclient-sys",
    "rusqlite", "sqlite", "libsqlite3-sys", "r2d2-postgres", "deadpool-postgres",
    "sea-orm", "sea-query", "rbatis", "rbdc", "rbdc-pg", "rbdc-mysql",
    "rbdc-sqlite", "tiberius", "odbc-api",
}
IGNORED_DIRS = {
    ".git", ".hg", ".svn", "target", "node_modules", ".venv", ".uv-cache",
    "__pycache__", "dist", "linters", "linters-extra", "worktrees", "references",
}
IGNORED_NODES = {
    "line_comment", "block_comment", "string_literal", "raw_string_literal",
    "char_literal", "byte_literal",
}


def rust_diagnostics(source: bytes, label: str) -> list[str]:
    pending = [PARSER.parse(source).root_node]
    diagnostics = []
    while pending:
        node = pending.pop()
        if node.type in IGNORED_NODES:
            continue
        pending.extend(reversed(node.children))
        if node.children:
            continue
        token = source[node.start_byte:node.end_byte].decode().removeprefix("r#")
        if token in FORBIDDEN:
            diagnostics.append(
                f"{label}:{node.start_point.row + 1}:{node.start_point.column + 1}: "
                f"SQL001: `{token}` bypasses typed Diesel query policy; "
                "use Diesel's typed query DSL (imports, tests and macros included)"
            )
    return diagnostics


def manifest_diagnostics(source: str, label: str) -> list[str]:
    try:
        manifest = tomllib.loads(source)
    except tomllib.TOMLDecodeError as error:
        return [f"{label}:1:1: SQL003: cannot inspect Cargo manifest: {error}"]
    diagnostics = []

    def visit(table: dict) -> None:
        for key, value in table.items():
            if key in {"dependencies", "dev-dependencies", "build-dependencies"}:
                for alias, specification in value.items():
                    package = specification.get("package", alias) if isinstance(specification, dict) else alias
                    if package in ALTERNATE_DRIVERS or alias in ALTERNATE_DRIVERS:
                        diagnostics.append(
                            f"{label}:1:1: SQL002: dependency `{alias}` (package `{package}`) "
                            "is an alternate SQL driver; use typed Diesel"
                        )
            if isinstance(value, dict):
                visit(value)

    visit(manifest)
    return diagnostics


def check_root(root: Path) -> list[str]:
    if not root.is_dir():
        return [f"{root}:1:1: SQL003: scan root does not exist"]
    diagnostics = []
    # Do not use production_source: cfg(test), mocks, generated schema, and
    # proc-macro token trees must all remain visible to this policy.
    for directory, dirs, files in os.walk(root):
        dirs[:] = sorted(name for name in dirs if name not in IGNORED_DIRS)
        for name in sorted(files):
            path = Path(directory) / name
            label = str(path.relative_to(root))
            if path.suffix == ".rs":
                diagnostics.extend(rust_diagnostics(path.read_bytes(), label))
            if name == "Cargo.toml":
                diagnostics.extend(manifest_diagnostics(path.read_text(), label))
    return diagnostics


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        suite = unittest.defaultTestLoader.discover(str(Path(__file__).parent), pattern="test_check.py")
        return 0 if unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful() else 1
    diagnostics = check_root(args.root.resolve())
    if diagnostics:
        print("\n".join(diagnostics), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
