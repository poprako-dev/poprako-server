#!/usr/bin/env -S uv run --script
# /// script
# dependencies = []
# ///

"""Enforce the API shape contract for PopRaKo data and HTTP boundaries."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


ROOT = Path(__file__).parents[2]

FIELD_PATTERN = re.compile(r"pub\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*:\s*([^\n,]+)")
TUPLE_STRUCT_PATTERN = re.compile(
    r"pub\s+struct\s+([A-Za-z_][A-Za-z0-9_]*)\s*(?:<[^>]*>)?\s*\("
)
NAMED_STRUCT_PATTERN = re.compile(
    r"pub\s+struct\s+([A-Za-z_][A-Za-z0-9_]*)\s*(?:<[^>]*>)?\s*\{([^}]*)\}"
)
ALIAS_PATTERN = re.compile(
    r"pub\s+type\s+([A-Za-z_][A-Za-z0-9_]*)\s*=\s*([^;]+);"
)
PATH_ATTRIBUTE_PATTERN = re.compile(
    r'#\[cfg_attr\(feature = "swagger", utoipa::path\([\s\S]*?\)\)\]'
)
HANDLER_PATTERN = re.compile(
    r"pub\s+async\s+fn\s+([A-Za-z_][A-Za-z0-9_]*)[\s\S]*?\)\s*->\s*([^{]+?)\s*\{"
)
MAP_TYPE_PATTERN = re.compile(r"\b(BTreeMap|HashMap)\s*<")


def line_column(text: str, offset: int) -> tuple[int, int]:
    line = text.count("\n", 0, offset) + 1
    column = offset - text.rfind("\n", 0, offset)
    return line, column


def diagnostic(
    path: Path,
    root: Path,
    text: str,
    offset: int,
    code: str,
    message: str,
) -> str:
    line, column = line_column(text, offset)
    return f"{path.relative_to(root)}:{line}:{column}: {code}: {message}"


def production_files(directory: Path) -> list[Path]:
    if not directory.is_dir():
        return []

    return [
        path
        for path in sorted(directory.rglob("*.rs"))
        if "tests" not in path.relative_to(directory).parts
        and not path.name.endswith("tests.rs")
    ]


def check_data(root: Path) -> list[str]:
    diagnostics: list[str] = []

    for path in production_files(root / "src" / "data"):
        text = path.read_text()

        for match in TUPLE_STRUCT_PATTERN.finditer(text):
            diagnostics.append(diagnostic(
                path,
                root,
                text,
                match.start(),
                "SHAPE001",
                f"DTO '{match.group(1)}' must use named fields instead of a tuple struct",
            ))

        for match in NAMED_STRUCT_PATTERN.finditer(text):
            fields = match.group(2)

            for field in FIELD_PATTERN.finditer(fields):
                if MAP_TYPE_PATTERN.search(field.group(2)):
                    diagnostics.append(diagnostic(
                        path,
                        root,
                        text,
                        match.start(2) + field.start(),
                        "SHAPE002",
                        f"DTO field '{field.group(1)}' must use named entries instead of a raw map",
                    ))

        for match in ALIAS_PATTERN.finditer(text):
            if MAP_TYPE_PATTERN.search(match.group(2)):
                diagnostics.append(diagnostic(
                    path,
                    root,
                    text,
                    match.start(),
                    "SHAPE003",
                    f"DTO alias '{match.group(1)}' must not expose a raw map",
                ))

    return diagnostics


def check_handlers(root: Path) -> list[str]:
    diagnostics: list[str] = []
    directory = root / "src" / "api" / "http" / "handler"

    for path in production_files(directory):
        text = path.read_text()

        for attribute in PATH_ATTRIBUTE_PATTERN.finditer(text):
            handler = HANDLER_PATTERN.search(text, attribute.end())

            if handler is None:
                continue

            return_type = " ".join(handler.group(2).split())

            if return_type.startswith("HttpResult<"):
                continue

            if return_type == "HttpNoContent":
                continue

            diagnostics.append(diagnostic(
                path,
                root,
                text,
                handler.start(),
                "HTTP001",
                f"handler '{handler.group(1)}' must return HttpResult or HttpNoContent, found {return_type}",
            ))

    return diagnostics


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    args = parser.parse_args()

    root = args.root.resolve()
    diagnostics = [*check_data(root), *check_handlers(root)]

    for entry in diagnostics:
        print(entry)

    return 1 if diagnostics else 0


if __name__ == "__main__":
    sys.exit(main())
