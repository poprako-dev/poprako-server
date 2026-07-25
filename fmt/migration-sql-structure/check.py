#!/usr/bin/env python3

"""Enforce single-table, single-responsibility SQL migrations."""

from __future__ import annotations

import argparse
import re
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).parents[2]
EXEMPT_MIGRATIONS = {
    "00000000000000_diesel_initial_setup",
    "2026-07-17-083438-0000_enable-features",
}
CREATE_KIND = re.compile(r"create-(?P<table>[a-z0-9-]+)-table$")
INDEX_KIND = re.compile(r"index-(?P<table>[a-z0-9-]+)-table$")
SEED_KIND = re.compile(r"seed-[a-z0-9-]+$")
CREATE_TABLE = re.compile(
    r'^CREATE\s+TABLE(?:\s+IF\s+NOT\s+EXISTS)?\s+"?(?P<table>t_[a-z0-9_]+)"?',
    re.IGNORECASE | re.DOTALL,
)
DROP_TABLE = re.compile(
    r'^DROP\s+TABLE(?:\s+IF\s+EXISTS)?\s+"?(?P<table>t_[a-z0-9_]+)"?',
    re.IGNORECASE | re.DOTALL,
)
CREATE_INDEX = re.compile(
    r'^CREATE\s+(?:UNIQUE\s+)?INDEX(?:\s+IF\s+NOT\s+EXISTS)?\s+'
    r'"?(?P<index>[a-z0-9_]+)"?\s+ON\s+"?(?P<table>t_[a-z0-9_]+)"?',
    re.IGNORECASE | re.DOTALL,
)
DROP_INDEX = re.compile(
    r'^DROP\s+INDEX(?:\s+IF\s+EXISTS)?\s+"?(?P<index>[a-z0-9_]+)"?',
    re.IGNORECASE | re.DOTALL,
)
INSERT_INTO = re.compile(
    r'^INSERT\s+INTO\s+"?(?P<table>t_[a-z0-9_]+)"?',
    re.IGNORECASE | re.DOTALL,
)
DELETE_FROM = re.compile(
    r'^DELETE\s+FROM\s+"?(?P<table>t_[a-z0-9_]+)"?',
    re.IGNORECASE | re.DOTALL,
)


def migration_kind(path: Path) -> str:
    return path.name.partition("_")[2]


def mask_comments(source: str) -> str:
    chars = list(source)
    index = 0
    quote: str | None = None

    while index < len(chars):
        current = chars[index]
        following = chars[index + 1] if index + 1 < len(chars) else ""

        if quote is not None:
            if current == quote:
                if index + 1 < len(chars) and chars[index + 1] == quote:
                    index += 2
                    continue

                quote = None

            index += 1
            continue

        if current in {"'", '"'}:
            quote = current
            index += 1
            continue

        if current == "-" and following == "-":
            while index < len(chars) and chars[index] != "\n":
                chars[index] = " "
                index += 1

            continue

        if current == "/" and following == "*":
            chars[index] = " "
            chars[index + 1] = " "
            index += 2

            while index + 1 < len(chars):
                if chars[index] == "*" and chars[index + 1] == "/":
                    chars[index] = " "
                    chars[index + 1] = " "
                    index += 2
                    break

                if chars[index] != "\n":
                    chars[index] = " "

                index += 1

            continue

        index += 1

    return "".join(chars)


def statements(path: Path) -> list[tuple[str, int]]:
    source = mask_comments(path.read_text())
    found: list[tuple[str, int]] = []
    start = 0
    quote: str | None = None
    index = 0

    while index < len(source):
        current = source[index]

        if quote is not None:
            if current == quote:
                if index + 1 < len(source) and source[index + 1] == quote:
                    index += 2
                    continue

                quote = None

            index += 1
            continue

        if current in {"'", '"'}:
            quote = current
        elif current == ";":
            statement = source[start:index].strip()

            if statement:
                line = source.count("\n", 0, start) + 1
                found.append((statement, line))

            start = index + 1

        index += 1

    trailing = source[start:].strip()

    if trailing:
        line = source.count("\n", 0, start) + 1
        found.append((trailing, line))

    return found


def diagnostic(path: Path, root: Path, line: int, code: str, message: str) -> str:
    return f"{path.relative_to(root)}:{line}: {code}: {message}"


def expected_table(match: re.Match[str]) -> str:
    return "t_" + match.group("table").replace("-", "_")


def check_create(path: Path, root: Path, match: re.Match[str]) -> list[str]:
    up_path = path / "up.sql"
    down_path = path / "down.sql"
    up_statements = statements(up_path)
    down_statements = statements(down_path)
    expected = expected_table(match)
    diagnostics: list[str] = []

    up_matches = [CREATE_TABLE.match(statement) for statement, _ in up_statements]
    down_matches = [DROP_TABLE.match(statement) for statement, _ in down_statements]

    if (
        len(up_statements) != 1
        or up_matches[0] is None
        or up_matches[0].group("table").lower() != expected
    ):
        diagnostics.append(
            diagnostic(
                up_path,
                root,
                up_statements[0][1] if up_statements else 1,
                "MIG002",
                f"create migration must contain only CREATE TABLE {expected}",
            ),
        )

    if (
        len(down_statements) != 1
        or down_matches[0] is None
        or down_matches[0].group("table").lower() != expected
    ):
        diagnostics.append(
            diagnostic(
                down_path,
                root,
                down_statements[0][1] if down_statements else 1,
                "MIG005",
                f"create migration down.sql must contain only DROP TABLE {expected}",
            ),
        )

    return diagnostics


def check_index(path: Path, root: Path, match: re.Match[str]) -> list[str]:
    up_path = path / "up.sql"
    down_path = path / "down.sql"
    up_statements = statements(up_path)
    down_statements = statements(down_path)
    expected = expected_table(match)
    up_matches = [CREATE_INDEX.match(statement) for statement, _ in up_statements]
    down_matches = [DROP_INDEX.match(statement) for statement, _ in down_statements]
    diagnostics: list[str] = []

    valid_up = (
        bool(up_matches)
        and all(index_match is not None for index_match in up_matches)
        and all(
            index_match.group("table").lower() == expected
            for index_match in up_matches
            if index_match is not None
        )
    )

    if not valid_up:
        diagnostics.append(
            diagnostic(
                up_path,
                root,
                up_statements[0][1] if up_statements else 1,
                "MIG003",
                f"index migration must contain only indexes on {expected}",
            ),
        )

    valid_down = bool(down_matches) and all(
        index_match is not None for index_match in down_matches
    )
    up_indexes = {
        index_match.group("index").lower()
        for index_match in up_matches
        if index_match is not None
    }
    down_indexes = {
        index_match.group("index").lower()
        for index_match in down_matches
        if index_match is not None
    }

    if not valid_down or up_indexes != down_indexes:
        diagnostics.append(
            diagnostic(
                down_path,
                root,
                down_statements[0][1] if down_statements else 1,
                "MIG005",
                "index migration down.sql must drop exactly the indexes created by up.sql",
            ),
        )

    return diagnostics


def check_seed(path: Path, root: Path) -> list[str]:
    up_path = path / "up.sql"
    down_path = path / "down.sql"
    up_statements = statements(up_path)
    down_statements = statements(down_path)
    up_matches = [INSERT_INTO.match(statement) for statement, _ in up_statements]
    down_matches = [DELETE_FROM.match(statement) for statement, _ in down_statements]
    up_tables = {
        insert_match.group("table").lower()
        for insert_match in up_matches
        if insert_match is not None
    }
    down_tables = {
        delete_match.group("table").lower()
        for delete_match in down_matches
        if delete_match is not None
    }
    diagnostics: list[str] = []

    if (
        not up_matches
        or not all(insert_match is not None for insert_match in up_matches)
        or len(up_tables) != 1
    ):
        diagnostics.append(
            diagnostic(
                up_path,
                root,
                up_statements[0][1] if up_statements else 1,
                "MIG004",
                "seed migration must contain only INSERT statements for one table",
            ),
        )

    if (
        not down_matches
        or not all(delete_match is not None for delete_match in down_matches)
        or down_tables != up_tables
    ):
        diagnostics.append(
            diagnostic(
                down_path,
                root,
                down_statements[0][1] if down_statements else 1,
                "MIG005",
                "seed migration down.sql must delete from the same single table",
            ),
        )

    return diagnostics


def check_migration(path: Path, root: Path) -> list[str]:
    if path.name in EXEMPT_MIGRATIONS:
        return []

    up_path = path / "up.sql"
    down_path = path / "down.sql"

    if not up_path.is_file() or not down_path.is_file():
        return [
            diagnostic(
                up_path if not up_path.is_file() else down_path,
                root,
                1,
                "MIG001",
                "migration must contain both up.sql and down.sql",
            ),
        ]

    kind = migration_kind(path)
    create_match = CREATE_KIND.fullmatch(kind)

    if create_match is not None:
        return check_create(path, root, create_match)

    index_match = INDEX_KIND.fullmatch(kind)

    if index_match is not None:
        return check_index(path, root, index_match)

    if SEED_KIND.fullmatch(kind) is not None:
        return check_seed(path, root)

    return [
        diagnostic(
            up_path,
            root,
            1,
            "MIG001",
            "unclassified or patch-style business migration is forbidden",
        ),
    ]


def check_root(root: Path) -> list[str]:
    migration_root = root / "migrations"

    if not migration_root.is_dir():
        return []

    return [
        item
        for path in sorted(migration_root.iterdir())
        if path.is_dir()
        for item in check_migration(path, root)
    ]


def write_migration(root: Path, name: str, up_sql: str, down_sql: str) -> None:
    migration = root / "migrations" / name
    migration.mkdir(parents=True)
    (migration / "up.sql").write_text(up_sql)
    (migration / "down.sql").write_text(down_sql)


def self_test() -> int:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        write_migration(
            root,
            "2026-01-01-000000-0000_create-widget-table",
            'CREATE TABLE "t_widget" ("f_id" TEXT PRIMARY KEY);\n',
            'DROP TABLE "t_widget";\n',
        )
        write_migration(
            root,
            "2026-01-01-000000-0001_index-widget-table",
            'CREATE INDEX "idx_widget_id" ON "t_widget" ("f_id");\n',
            'DROP INDEX "idx_widget_id";\n',
        )
        write_migration(
            root,
            "2026-01-01-000000-0002_seed-widget",
            'INSERT INTO "t_widget" ("f_id") VALUES (\'widget-1\');\n',
            'DELETE FROM "t_widget" WHERE "f_id" = \'widget-1\';\n',
        )

        if check_root(root):
            print("self-test: valid migration fixtures were rejected", file=sys.stderr)
            print("\n".join(check_root(root)), file=sys.stderr)
            return 1

        write_migration(
            root,
            "2026-01-01-000000-0003_patch-widget-index",
            'DROP INDEX "idx_widget_id";\n',
            'CREATE INDEX "idx_widget_id" ON "t_widget" ("f_id");\n',
        )
        write_migration(
            root,
            "2026-01-01-000000-0004_create-mixed-table",
            'CREATE TABLE "t_mixed" ("f_id" TEXT PRIMARY KEY);\n'
            'CREATE INDEX "idx_mixed_id" ON "t_mixed" ("f_id");\n',
            'DROP TABLE "t_mixed";\n',
        )
        write_migration(
            root,
            "2026-01-01-000000-0005_index-widget-table",
            'CREATE INDEX "idx_other_id" ON "t_other" ("f_id");\n',
            'DROP INDEX "idx_missing";\n',
        )
        write_migration(
            root,
            "2026-01-01-000000-0006_seed-mixed",
            'INSERT INTO "t_widget" ("f_id") VALUES (\'widget-2\');\n'
            'INSERT INTO "t_other" ("f_id") VALUES (\'other-1\');\n',
            'DELETE FROM "t_widget";\n',
        )

        codes = {
            match.group(1)
            for item in check_root(root)
            if (match := re.search(r": (MIG\d{3}):", item)) is not None
        }

        if codes != {"MIG001", "MIG002", "MIG003", "MIG004", "MIG005"}:
            print("self-test: invalid migrations were not fully rejected", file=sys.stderr)
            print("\n".join(check_root(root)), file=sys.stderr)
            return 1

    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    diagnostics = check_root(args.root.resolve())

    if diagnostics:
        print("\n".join(diagnostics), file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
