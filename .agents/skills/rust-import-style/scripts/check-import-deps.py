#!/usr/bin/env python3
from __future__ import annotations

import argparse
import re
import sys
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


@dataclass(frozen=True)
class UseEdge:
    source: tuple[str, ...]
    target: tuple[str, ...]
    file: Path
    line: int
    use_text: str
    expanded_from: str


def fmt_mod(path: tuple[str, ...]) -> str:
    return "crate" if not path else "crate::" + "::".join(path)


def read_crate_name(crate_root: Path) -> str | None:
    cargo_toml = crate_root / "Cargo.toml"

    if not cargo_toml.exists():
        return None

    in_package = False

    for raw in cargo_toml.read_text(encoding="utf-8", errors="ignore").splitlines():
        line = raw.strip()

        if not line or line.startswith("#"):
            continue

        if line.startswith("[") and line.endswith("]"):
            in_package = line == "[package]"
            continue

        if in_package and line.startswith("name"):
            m = re.match(r'name\s*=\s*"([^"]+)"', line)

            if m:
                return m.group(1).replace("-", "_")

    return None


def file_to_mod_path(src_dir: Path, file: Path) -> tuple[str, ...] | None:
    rel = file.relative_to(src_dir)

    if rel.parts and rel.parts[0] == "bin":
        return None

    if rel.name in {"lib.rs", "main.rs"} and len(rel.parts) == 1:
        return ()

    parts = list(rel.parts)

    if parts[-1] == "mod.rs":
        parts = parts[:-1]
    else:
        parts[-1] = Path(parts[-1]).stem

    return tuple(parts)


def discover_modules(src_dir: Path) -> dict[tuple[str, ...], Path]:
    modules: dict[tuple[str, ...], Path] = {}

    for file in sorted(src_dir.rglob("*.rs")):
        mod_path = file_to_mod_path(src_dir, file)

        if mod_path is None:
            continue

        modules.setdefault(mod_path, file)

    return modules


def mask_comments_and_strings(text: str) -> str:
    out = list(text)
    i = 0
    n = len(text)

    def blank(a: int, b: int) -> None:
        for k in range(a, min(b, n)):
            if out[k] != "\n":
                out[k] = " "

    while i < n:
        if text.startswith("//", i):
            j = text.find("\n", i + 2)
            j = n if j == -1 else j
            blank(i, j)
            i = j
            continue

        if text.startswith("/*", i):
            depth = 1
            j = i + 2

            while j < n and depth > 0:
                if text.startswith("/*", j):
                    depth += 1
                    j += 2
                elif text.startswith("*/", j):
                    depth -= 1
                    j += 2
                else:
                    j += 1

            blank(i, j)
            i = j
            continue

        if text.startswith("br", i) or text.startswith("rb", i) or text.startswith("r", i):
            if text.startswith("br", i) or text.startswith("rb", i):
                j = i + 2
            else:
                j = i + 1

            hashes = 0

            while j < n and text[j] == "#":
                hashes += 1
                j += 1

            if j < n and text[j] == '"':
                end_pat = '"' + ("#" * hashes)
                end = text.find(end_pat, j + 1)
                end = n if end == -1 else end + len(end_pat)
                blank(i, end)
                i = end
                continue

        if text[i] == '"' or text.startswith('b"', i):
            j = i + 2 if text.startswith('b"', i) else i + 1
            escaped = False

            while j < n:
                ch = text[j]

                if escaped:
                    escaped = False
                elif ch == "\\":
                    escaped = True
                elif ch == '"':
                    j += 1
                    break

                j += 1

            blank(i, j)
            i = j
            continue

        i += 1

    return "".join(out)


USE_RE = re.compile(
    r"""
    (?<![A-Za-z0-9_])
    (?:pub\s*(?:\([^)]*\)\s*)?)?
    use
    \s+
    (?P<body>.*?)
    ;
    """,
    re.DOTALL | re.VERBOSE,
)


def collect_use_statements(text: str) -> list[tuple[int, str, str]]:
    masked = mask_comments_and_strings(text)
    result: list[tuple[int, str, str]] = []

    for m in USE_RE.finditer(masked):
        line = text.count("\n", 0, m.start()) + 1
        body = m.group("body")
        use_text = " ".join(text[m.start() : m.end()].split())
        result.append((line, body, use_text))

    return result


def split_top_level_commas(s: str) -> list[str]:
    parts: list[str] = []
    start = 0
    depth = 0

    for i, ch in enumerate(s):
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
        elif ch == "," and depth == 0:
            part = s[start:i].strip()

            if part:
                parts.append(part)

            start = i + 1

    tail = s[start:].strip()

    if tail:
        parts.append(tail)

    return parts


def find_top_lbrace(s: str) -> int:
    depth = 0

    for i, ch in enumerate(s):
        if ch == "{":
            if depth == 0:
                return i

            depth += 1
        elif ch == "}":
            depth -= 1

    return -1


def find_matching_rbrace(s: str, lbrace: int) -> int:
    depth = 0

    for i in range(lbrace, len(s)):
        if s[i] == "{":
            depth += 1
        elif s[i] == "}":
            depth -= 1

            if depth == 0:
                return i

    return -1


def strip_alias(s: str) -> str:
    return re.split(r"\s+as\s+", s, maxsplit=1)[0].strip()


def join_path(prefix: str, suffix: str) -> str:
    prefix = prefix.strip().strip(":")
    suffix = suffix.strip().strip(":")

    if not prefix:
        return suffix

    if not suffix:
        return prefix

    return prefix + "::" + suffix


def expand_use_tree(expr: str, prefix: str = "") -> list[str]:
    expr = expr.strip()

    if expr.startswith("::"):
        expr = expr[2:]

    result: list[str] = []

    for part in split_top_level_commas(expr):
        lbrace = find_top_lbrace(part)

        if lbrace >= 0:
            rbrace = find_matching_rbrace(part, lbrace)

            if rbrace < 0:
                continue

            before = part[:lbrace].strip()

            while before.endswith(":"):
                before = before[:-1].rstrip()

            inside = part[lbrace + 1 : rbrace]
            result.extend(expand_use_tree(inside, join_path(prefix, before)))
            continue

        leaf = strip_alias(part)

        if leaf:
            result.append(join_path(prefix, leaf))

    return result


def resolve_absolute_path(
    source: tuple[str, ...],
    use_path: str,
    top_modules: set[str],
    crate_name: str | None,
    allow_implicit_crate: bool,
) -> tuple[str, ...] | None:
    parts = [p.strip() for p in use_path.split("::") if p.strip()]
    parts = [p for p in parts if p != "*"]

    if not parts:
        return None

    first = parts[0]
    rest = parts[1:]

    if first == "crate":
        base: list[str] = []
    elif crate_name is not None and first == crate_name:
        base = []
    elif first == "self":
        base = list(source)
    elif first == "super":
        base = list(source[:-1])

        while rest and rest[0] == "super":
            if base:
                base.pop()

            rest = rest[1:]
    elif allow_implicit_crate and first in top_modules:
        base = []
        rest = parts
    else:
        return None

    out = base[:]

    for seg in rest:
        if seg in {"self", "*"}:
            continue

        if seg == "super":
            if out:
                out.pop()

            continue

        if seg.startswith("<"):
            return None

        out.append(seg)

    return tuple(out)


def resolve_target_module(
    absolute_path: tuple[str, ...],
    known_modules: set[tuple[str, ...]],
) -> tuple[str, ...] | None:
    for i in range(len(absolute_path), 0, -1):
        candidate = absolute_path[:i]

        if candidate in known_modules:
            return candidate

    return None


def is_strict_ancestor(
    maybe_ancestor: tuple[str, ...],
    node: tuple[str, ...],
) -> bool:
    return (
        len(maybe_ancestor) < len(node)
        and node[: len(maybe_ancestor)] == maybe_ancestor
    )


def is_allowed_dependency(
    source: tuple[str, ...],
    target: tuple[str, ...],
) -> bool:
    if source == target:
        return True

    if is_strict_ancestor(target, source):
        return False

    return True


def collect_edges(
    modules: dict[tuple[str, ...], Path],
    src_dir: Path,
    crate_name: str | None,
    allow_implicit_crate: bool,
) -> tuple[list[UseEdge], list[UseEdge]]:
    known_modules = set(modules)
    top_modules = {m[0] for m in known_modules if m}

    all_edges: list[UseEdge] = []
    illegal_edges: list[UseEdge] = []

    for source, file in modules.items():
        text = file.read_text(encoding="utf-8", errors="ignore")

        for line, body, use_text in collect_use_statements(text):
            for expanded in expand_use_tree(body):
                absolute_path = resolve_absolute_path(
                    source=source,
                    use_path=expanded,
                    top_modules=top_modules,
                    crate_name=crate_name,
                    allow_implicit_crate=allow_implicit_crate,
                )

                if absolute_path is None:
                    continue

                target = resolve_target_module(absolute_path, known_modules)

                if target is None:
                    continue

                edge = UseEdge(
                    source=source,
                    target=target,
                    file=file.relative_to(src_dir.parent),
                    line=line,
                    use_text=use_text,
                    expanded_from=expanded,
                )

                if source and source != target:
                    all_edges.append(edge)

                if not is_allowed_dependency(source, target):
                    illegal_edges.append(edge)

    return all_edges, illegal_edges


def strongly_connected_components(
    nodes: Iterable[tuple[str, ...]],
    graph: dict[tuple[str, ...], set[tuple[str, ...]]],
) -> list[list[tuple[str, ...]]]:
    index = 0
    stack: list[tuple[str, ...]] = []
    on_stack: set[tuple[str, ...]] = set()
    indices: dict[tuple[str, ...], int] = {}
    lowlinks: dict[tuple[str, ...], int] = {}
    components: list[list[tuple[str, ...]]] = []

    def visit(v: tuple[str, ...]) -> None:
        nonlocal index

        indices[v] = index
        lowlinks[v] = index
        index += 1
        stack.append(v)
        on_stack.add(v)

        for w in graph.get(v, set()):
            if w not in indices:
                visit(w)
                lowlinks[v] = min(lowlinks[v], lowlinks[w])
            elif w in on_stack:
                lowlinks[v] = min(lowlinks[v], indices[w])

        if lowlinks[v] == indices[v]:
            comp: list[tuple[str, ...]] = []

            while True:
                w = stack.pop()
                on_stack.remove(w)
                comp.append(w)

                if w == v:
                    break

            if len(comp) > 1 or v in graph.get(v, set()):
                components.append(sorted(comp))

    for node in sorted(nodes):
        if node not in indices:
            visit(node)

    return components


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "crate_root",
        nargs="?",
        default=".",
        help="Rust crate root, default: current directory",
    )
    parser.add_argument(
        "--src",
        default="src",
        help="source directory relative to crate root, default: src",
    )
    parser.add_argument(
        "--no-implicit-crate",
        action="store_true",
        help="only check crate:: / self:: / super:: / crate-name paths",
    )

    args = parser.parse_args()

    crate_root = Path(args.crate_root).resolve()
    src_dir = (crate_root / args.src).resolve()

    if not src_dir.exists():
        print(f"error: source directory does not exist: {src_dir}", file=sys.stderr)
        return 2

    modules = discover_modules(src_dir)
    crate_name = read_crate_name(crate_root)

    edges, illegal_edges = collect_edges(
        modules=modules,
        src_dir=src_dir,
        crate_name=crate_name,
        allow_implicit_crate=not args.no_implicit_crate,
    )

    graph: dict[tuple[str, ...], set[tuple[str, ...]]] = defaultdict(set)
    nodes = set(modules)

    for edge in edges:
        graph[edge.source].add(edge.target)
        nodes.add(edge.source)
        nodes.add(edge.target)

    cycles = strongly_connected_components(nodes, graph)

    if illegal_edges:
        print("Illegal internal module dependencies:")

        for edge in illegal_edges:
            print(
                f"  {edge.file}:{edge.line}: "
                f"{fmt_mod(edge.source)} -> {fmt_mod(edge.target)}"
            )
            print(f"    use: {edge.use_text}")
            print(f"    expanded: {edge.expanded_from}")

    if cycles:
        if illegal_edges:
            print()

        print("Cyclic internal module dependencies:")

        edge_map: dict[tuple[tuple[str, ...], tuple[str, ...]], list[UseEdge]] = defaultdict(list)

        for edge in edges:
            edge_map[(edge.source, edge.target)].append(edge)

        for i, comp in enumerate(cycles, start=1):
            comp_set = set(comp)

            print(f"  cycle group {i}: " + ", ".join(fmt_mod(m) for m in comp))

            for src in comp:
                for dst in sorted(graph.get(src, set())):
                    if dst not in comp_set:
                        continue

                    sample = edge_map[(src, dst)][0]

                    print(
                        f"    {fmt_mod(src)} -> {fmt_mod(dst)} "
                        f"at {sample.file}:{sample.line}"
                    )

    if illegal_edges or cycles:
        return 1

    print("OK: no illegal internal module dependency or cycle found.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
