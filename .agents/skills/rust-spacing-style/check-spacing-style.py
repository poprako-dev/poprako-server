#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# ///

from __future__ import annotations

import argparse
import bisect
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path


IGNORED_DIRS = {
    ".git",
    ".hg",
    ".svn",
    ".idea",
    ".vscode",
    "target",
    "node_modules",
}


@dataclass(frozen=True)
class Statement:
    start_i: int
    end_i: int
    start_line: int
    start_col: int
    end_line: int
    end_col: int


@dataclass
class Block:
    open_i: int
    open_line: int
    open_col: int
    is_code: bool
    check_start_separator: bool
    kind: str

    statements: list[Statement] = field(default_factory=list)

    current_start_i: int | None = None
    current_start_line: int = 0
    current_start_col: int = 0

    paren_depth: int = 0
    bracket_depth: int = 0


@dataclass(frozen=True)
class Diagnostic:
    path: Path
    line: int
    col: int
    code: str
    message: str
    remove_lines: tuple[int, ...]


def is_ident_char(ch: str) -> bool:
    return ch.isalnum() or ch == "_"


def replace_non_newline(chars: list[str], start: int, end: int) -> None:
    for i in range(start, min(end, len(chars))):
        if chars[i] != "\n":
            chars[i] = " "


def raw_string_at(src: str, i: int) -> tuple[int, int] | None:
    if i > 0 and is_ident_char(src[i - 1]):
        return None

    for prefix in ("br", "rb", "cr", "r"):
        if not src.startswith(prefix, i):
            continue

        j = i + len(prefix)
        hashes = 0

        while j + hashes < len(src) and src[j + hashes] == "#":
            hashes += 1

        quote_i = j + hashes

        if quote_i < len(src) and src[quote_i] == '"':
            return quote_i, hashes

    return None


def sanitize(src: str) -> str:
    chars = list(src)
    i = 0
    n = len(src)

    while i < n:
        raw = raw_string_at(src, i)

        if raw is not None:
            quote_i, hashes = raw
            end_pat = '"' + ("#" * hashes)
            j = quote_i + 1
            end = n

            while j < n:
                if src.startswith(end_pat, j):
                    end = j + len(end_pat)
                    break

                j += 1

            replace_non_newline(chars, i, end)
            i = end
            continue

        if src.startswith("//", i):
            j = src.find("\n", i)

            if j == -1:
                j = n

            replace_non_newline(chars, i, j)
            i = j
            continue

        if src.startswith("/*", i):
            depth = 1
            j = i + 2

            while j < n and depth > 0:
                if src.startswith("/*", j):
                    depth += 1
                    j += 2
                elif src.startswith("*/", j):
                    depth -= 1
                    j += 2
                else:
                    j += 1

            replace_non_newline(chars, i, j)
            i = j
            continue

        ch = src[i]

        if ch == '"':
            j = i + 1
            escaped = False

            while j < n:
                c = src[j]

                if c == "\n" and not escaped:
                    break

                if escaped:
                    escaped = False
                elif c == "\\":
                    escaped = True
                elif c == '"':
                    j += 1
                    break

                j += 1

            replace_non_newline(chars, i, j)
            i = j
            continue

        if ch == "'" and i + 1 < n:
            # Lifetime: 'a, 'static, etc. Do not treat as char literal.
            if src[i + 1].isalpha() or src[i + 1] == "_":
                j = i + 2

                while j < n and is_ident_char(src[j]):
                    j += 1

                if j >= n or src[j] != "'":
                    i += 1
                    continue

            j = i + 1
            escaped = False

            while j < n:
                c = src[j]

                if c == "\n" and not escaped:
                    break

                if escaped:
                    escaped = False
                elif c == "\\":
                    escaped = True
                elif c == "'":
                    j += 1
                    break

                j += 1

            if j > i + 1:
                replace_non_newline(chars, i, j)
                i = j
                continue

        i += 1

    return "".join(chars)


def make_line_starts(src: str) -> list[int]:
    starts = [0]

    for i, ch in enumerate(src):
        if ch == "\n":
            starts.append(i + 1)

    return starts


def pos_to_line_col(line_starts: list[int], i: int) -> tuple[int, int]:
    if i < 0:
        i = 0

    line = bisect.bisect_right(line_starts, i)
    col = i - line_starts[line - 1] + 1

    return line, col


def prev_non_ws(src: str, i: int) -> int:
    j = i

    while j >= 0 and src[j].isspace():
        j -= 1

    return j


def skip_ws(src: str, i: int) -> int:
    while i < len(src) and src[i].isspace():
        i += 1

    return i


def prev_token(src: str, i: int) -> str:
    j = prev_non_ws(src, i - 1)

    if j < 0:
        return ""

    ch = src[j]

    if is_ident_char(ch):
        k = j

        while k >= 0 and is_ident_char(src[k]):
            k -= 1

        return src[k + 1 : j + 1]

    if ch == ">" and j > 0 and src[j - 1] == "=":
        return "=>"

    if ch == ":" and j > 0 and src[j - 1] == ":":
        return "::"

    return ch


def next_token(src: str, i: int) -> str:
    j = skip_ws(src, i)

    if j >= len(src):
        return ""

    ch = src[j]

    if is_ident_char(ch):
        k = j

        while k < len(src) and is_ident_char(src[k]):
            k += 1

        return src[j:k]

    if ch == "=" and j + 1 < len(src) and src[j + 1] == ">":
        return "=>"

    if ch == ":" and j + 1 < len(src) and src[j + 1] == ":":
        return "::"

    return ch


def normalized_prefix(prefix: str) -> str:
    s = prefix.lstrip()
    s = re.sub(r"^pub(?:\s*\([^)]*\))?\s+", "", s)
    s = re.sub(r"^async\s+", "", s)
    s = re.sub(r"^unsafe\s+", "", s)
    s = re.sub(r"^const\s+", "", s)

    return s


def first_word(prefix: str) -> str:
    s = normalized_prefix(prefix)
    m = re.match(r"([A-Za-z_]\w*)\b", s)

    return m.group(1) if m else ""


def looks_like_closure_prefix(prefix: str) -> bool:
    return re.search(r"\|[^|{};]*\|\s*$", prefix[-360:]) is not None


def looks_like_async_block(prefix: str) -> bool:
    return re.search(r"\basync\s+(?:move\s+)?$", prefix[-120:]) is not None


def classify_open_brace(clean: str, i: int, parent: Block) -> tuple[bool, bool, str]:
    prefix = clean[parent.current_start_i : i] if parent.current_start_i is not None else ""
    fw = first_word(prefix)
    prev = prev_token(clean, i)

    if fw == "fn":
        return True, True, "fn_body"

    if fw in {"impl", "trait", "mod", "extern"}:
        return True, False, "item_body"

    if fw in {"if", "else", "for", "while", "loop", "unsafe"} or prev == "else":
        return True, True, "control_body"

    if fw == "match":
        return True, True, "control_body"

    if looks_like_closure_prefix(prefix) or looks_like_async_block(prefix) or prev == "|":
        return True, True, "closure_body"

    if prev == "=>":
        return True, True, "match_arm_body"

    if prefix.strip() == "":
        return True, True, "block_expr"

    return False, False, "literal"


def control_block_continues(clean: str, after_close_i: int) -> bool:
    tok = next_token(clean, after_close_i)

    if tok in {"else", ".", "?", ";", ",", ")", "]", "=>"}:
        return True

    if tok in {"+", "-", "*", "/", "%", "&", "|", "^", "<", ">", "=", ":"}:
        return True

    return False


def start_statement(block: Block, i: int, line_starts: list[int]) -> None:
    if block.current_start_i is not None:
        return

    line, col = pos_to_line_col(line_starts, i)

    block.current_start_i = i
    block.current_start_line = line
    block.current_start_col = col


def add_statement(block: Block, clean: str, line_starts: list[int], end_i: int) -> None:
    if block.current_start_i is None:
        return

    real_end_i = prev_non_ws(clean, end_i)

    if real_end_i < block.current_start_i:
        block.current_start_i = None
        return

    raw = clean[block.current_start_i : real_end_i + 1].strip()

    if raw:
        end_line, end_col = pos_to_line_col(line_starts, real_end_i)

        block.statements.append(
            Statement(
                start_i=block.current_start_i,
                end_i=real_end_i,
                start_line=block.current_start_line,
                start_col=block.current_start_col,
                end_line=end_line,
                end_col=end_col,
            )
        )

    block.current_start_i = None
    block.current_start_line = 0
    block.current_start_col = 0
    block.paren_depth = 0
    block.bracket_depth = 0


def parse_blocks(src: str) -> tuple[list[Block], str]:
    clean = sanitize(src)
    line_starts = make_line_starts(src)

    root = Block(
        open_i=0,
        open_line=1,
        open_col=1,
        is_code=True,
        check_start_separator=False,
        kind="root",
    )

    stack: list[Block] = [root]
    closed: list[Block] = []
    i = 0

    while i < len(clean):
        ch = clean[i]
        top = stack[-1]

        if not top.is_code:
            if ch == "{":
                line, col = pos_to_line_col(line_starts, i)

                stack.append(
                    Block(
                        open_i=i,
                        open_line=line,
                        open_col=col,
                        is_code=False,
                        check_start_separator=False,
                        kind="literal",
                    )
                )
            elif ch == "}":
                closed.append(stack.pop())

            i += 1
            continue

        if ch.isspace():
            i += 1
            continue

        if ch == "}":
            add_statement(top, clean, line_starts, i - 1)

            popped = stack.pop()
            closed.append(popped)

            if stack:
                parent = stack[-1]

                if parent.is_code and parent.current_start_i is not None:
                    if popped.kind in {"control_body", "match_arm_body", "block_expr"}:
                        if not control_block_continues(clean, i + 1):
                            add_statement(parent, clean, line_starts, i)
                    elif popped.kind in {"fn_body", "item_body"}:
                        add_statement(parent, clean, line_starts, i)

            i += 1
            continue

        if ch not in {";", ","}:
            start_statement(top, i, line_starts)

        if ch == "{":
            is_code, check_start_separator, kind = classify_open_brace(clean, i, top)
            line, col = pos_to_line_col(line_starts, i)

            stack.append(
                Block(
                    open_i=i,
                    open_line=line,
                    open_col=col,
                    is_code=is_code,
                    check_start_separator=check_start_separator,
                    kind=kind,
                )
            )

            i += 1
            continue

        if ch == "(":
            top.paren_depth += 1
        elif ch == ")":
            if top.paren_depth > 0:
                top.paren_depth -= 1
        elif ch == "[":
            top.bracket_depth += 1
        elif ch == "]":
            if top.bracket_depth > 0:
                top.bracket_depth -= 1
        elif ch == ";" and top.paren_depth == 0 and top.bracket_depth == 0:
            add_statement(top, clean, line_starts, i)

        i += 1

    end_i = prev_non_ws(clean, len(clean) - 1)

    while stack:
        top = stack.pop()
        add_statement(top, clean, line_starts, end_i)
        closed.append(top)

    return closed, clean


def is_separator_comment_line(line: str) -> bool:
    return re.fullmatch(r"\s*//\s*", line) is not None


def is_empty_line(line: str) -> bool:
    return line.strip() == ""


def find_redundant_separators(path: Path) -> list[Diagnostic]:
    src = path.read_text(encoding="utf-8")
    lines = src.splitlines(keepends=False)
    blocks, _ = parse_blocks(src)

    diagnostics: list[Diagnostic] = []

    for block in blocks:
        if not block.check_start_separator:
            continue

        if len(block.statements) != 1:
            continue

        first = block.statements[0]

        # Lines between `{` and the first statement.
        start_line = block.open_line + 1
        end_line = first.start_line - 1

        if start_line > end_line:
            continue

        separator_lines: list[int] = []
        removable_lines: list[int] = []
        has_real_content = False

        for line_no in range(start_line, end_line + 1):
            line = lines[line_no - 1]

            if is_separator_comment_line(line):
                separator_lines.append(line_no)
                removable_lines.append(line_no)
                continue

            if is_empty_line(line):
                removable_lines.append(line_no)
                continue

            has_real_content = True

        if has_real_content:
            continue

        if not separator_lines:
            continue

        diagnostics.append(
            Diagnostic(
                path=path,
                line=separator_lines[0],
                col=1,
                code="BFX001",
                message=(
                    "redundant block-start separator comment in a single-statement block; "
                    "remove the empty `//` separator"
                ),
                remove_lines=tuple(removable_lines),
            )
        )

    return sorted(diagnostics, key=lambda d: (str(d.path), d.line, d.col))


def iter_rs_files(paths: list[Path]) -> list[Path]:
    files: list[Path] = []

    for path in paths:
        if path.is_file():
            if path.suffix == ".rs":
                files.append(path)

            continue

        if not path.is_dir():
            continue

        for child in path.rglob("*.rs"):
            if any(part in IGNORED_DIRS for part in child.parts):
                continue

            files.append(child)

    return sorted(set(files))


def apply_fixes(path: Path, diagnostics: list[Diagnostic]) -> bool:
    if not diagnostics:
        return False

    src = path.read_text(encoding="utf-8")
    keepends = src.splitlines(keepends=True)

    remove_line_set: set[int] = set()

    for diagnostic in diagnostics:
        remove_line_set.update(diagnostic.remove_lines)

    changed = False
    new_lines: list[str] = []

    for index, line in enumerate(keepends, start=1):
        if index in remove_line_set:
            changed = True
            continue

        new_lines.append(line)

    if changed:
        path.write_text("".join(new_lines), encoding="utf-8")

    return changed


def main() -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Find redundant empty `//` separator comments that were inserted "
            "before the first statement of a single-statement Rust block."
        )
    )

    parser.add_argument(
        "paths",
        nargs="*",
        type=Path,
        default=[Path(".")],
        help="Rust files or directories. Defaults to current directory.",
    )

    parser.add_argument(
        "--fix",
        action="store_true",
        help="Remove the redundant empty separator comment lines and adjacent empty lines in that separator region.",
    )

    args = parser.parse_args()

    all_diagnostics: list[Diagnostic] = []

    for path in iter_rs_files(args.paths):
        try:
            diagnostics = find_redundant_separators(path)
        except OSError as err:
            print(f"{path}: failed to read file: {err}", file=sys.stderr)
            return 2
        except UnicodeDecodeError as err:
            print(f"{path}: failed to read as utf-8: {err}", file=sys.stderr)
            return 2

        all_diagnostics.extend(diagnostics)

        if args.fix:
            apply_fixes(path, diagnostics)

    for diagnostic in sorted(all_diagnostics, key=lambda d: (str(d.path), d.line, d.col)):
        print(f"{diagnostic.path}:{diagnostic.line}:{diagnostic.col}: {diagnostic.code}: {diagnostic.message}")

    if args.fix and all_diagnostics:
        print(f"fixed {len(all_diagnostics)} redundant separator block(s)")

    return 1 if all_diagnostics else 0


if __name__ == "__main__":
    raise SystemExit(main())
