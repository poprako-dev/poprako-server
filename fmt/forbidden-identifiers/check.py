#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "tree-sitter==0.25.0",
#   "tree-sitter-rust==0.23.3",
# ]
# ///

"""Detect forbidden words in Rust identifier names.

Scans every Rust source file under src/ (excluding schema.rs) and reports
identifiers whose name segments contain words listed in docs/forbidden-words.md.
Also flags ``target_`` prefix.

Rules
-----

* ``error`` is always forbidden (FBD003).
* ``err`` is the replacement for ``error``, but its form is restricted (FBD004):

  - Function names: only ``_err`` **suffix** is allowed (e.g. ``parse_err``).
  - Local variables / parameters: only ``err_`` **prefix** is allowed, AND the
    binding must NOT be an Error type.
  - Every other ``err`` form (bare ``err``, middle-segment, const, static,
    field, enum variant) is forbidden.

* Identifiers inside ``#[cfg(test)]`` modules are skipped entirely.
"""

from __future__ import annotations

import argparse
import re
import sys
import tempfile
from pathlib import Path

import tree_sitter
import tree_sitter_rust

sys.path.insert(0, str(Path(__file__).parents[1]))
from production_source import production_source


ROOT = Path(__file__).parents[2]
PARSER = tree_sitter.Parser(tree_sitter.Language(tree_sitter_rust.language()))

# ---------------------------------------------------------------------------
# declaration kinds whose `name` field is a definition site
# ---------------------------------------------------------------------------

DECLARATION_KINDS: tuple[str, ...] = (
    "const_item",
    "enum_item",
    "enum_variant",
    "field_declaration",
    "function_item",
    "function_signature_item",
    "static_item",
    "struct_item",
    "trait_item",
    "type_item",
    "union_item",
)

# ---------------------------------------------------------------------------
# forbidden word segments
# ---------------------------------------------------------------------------

FORBIDDEN_SEGMENTS: dict[str, tuple[str, str]] = {
    "result":     ("FBD001", "'result' is forbidden — name what the value represents"),
    "res":        ("FBD002", "'res' is a forbidden abbreviation of 'result'"),
    "error":      ("FBD003", "'error' is forbidden — use 'err' instead"),
    "closure":    ("FBD005", "'closure' is a forbidden word"),
    "connection": ("FBD006", "'connection' is forbidden — use 'conn'"),
    "txn":        ("FBD007", "'txn' is a forbidden abbreviation of 'transaction'"),
    "tx":         ("FBD008", "'tx' is a forbidden abbreviation of 'transaction'"),
    "extension":  ("FBD010", "'extension' is forbidden — use 'ext' instead"),
    "previous":   ("FBD011", "'previous' is forbidden — use 'prev' instead"),
}


# ---------------------------------------------------------------------------
# helpers
# ---------------------------------------------------------------------------

def text(source: bytes, node: tree_sitter.Node) -> str:
    return source[node.start_byte : node.end_byte].decode()


def rust_files(root: Path) -> list[Path]:
    return sorted(
        path
        for path in (root / "src").rglob("*.rs")
        if path.name != "schema.rs"
    )


def resolve_ignore_files(
    root: Path,
    ignore_files: list[Path],
    ignore_lists: list[Path],
) -> set[Path]:
    """Resolve direct ignore paths and newline-delimited ignore lists."""
    configured_files = list(ignore_files)

    for ignore_list in ignore_lists:
        ignore_list_path = (
            ignore_list
            if ignore_list.is_absolute()
            else root / ignore_list
        )

        for line in ignore_list_path.read_text().splitlines():
            entry = line.strip()

            if entry and not entry.startswith("#"):
                configured_files.append(Path(entry))

    return {
        (path if path.is_absolute() else root / path).resolve()
        for path in configured_files
    }


def split_identifier(name: str) -> list[str]:
    """Split snake_case, SCREAMING_SNAKE_CASE, PascalCase, or camelCase into
    lowercase word segments."""

    if "_" in name:
        return name.lower().split("_")

    # PascalCase / camelCase:
    #   IOError   -> IO_Error   -> io, error
    #   ParseErr  -> Parse_Err  -> parse, err
    #   XMLParser -> XML_Parser -> xml, parser
    s = re.sub(r"([A-Z]+)([A-Z][a-z])", r"\1_\2", name)
    s = re.sub(r"([a-z\d])([A-Z])", r"\1_\2", s)

    return s.lower().split("_")


def segments_contain_error(segments: list[str]) -> bool:
    """Return True when *segments* contains ``error`` as a PascalCase word."""
    return "error" in segments


def location(path: Path, root: Path, node: tree_sitter.Node) -> str:
    return f"{path.relative_to(root)}:{node.start_point.row + 1}"


def error_line(
    path: Path,
    root: Path,
    node: tree_sitter.Node,
    code: str,
    message: str,
) -> str:
    return f"{location(path, root, node)}: {code}: {message}"


# ---------------------------------------------------------------------------
# cfg(test) detection
# ---------------------------------------------------------------------------

def _leading_attributes(
    node: tree_sitter.Node,
    source: bytes,
) -> list[tree_sitter.Node]:
    parent = node.parent

    if parent is None:
        return []

    index = next(
        (
            i
            for i, sib in enumerate(parent.children)
            if sib.start_byte == node.start_byte and sib.end_byte == node.end_byte
        ),
        None,
    )

    if index is None:
        return []

    attrs: list[tree_sitter.Node] = []

    for sib in reversed(parent.children[:index]):
        if sib.type == "attribute_item":
            attrs.append(sib)
            continue

        if not sib.is_named:
            continue

        break

    attrs.reverse()

    return attrs


def _is_cfg_test_attr(attr: tree_sitter.Node, source: bytes) -> bool:
    raw = text(source, attr)

    return re.fullmatch(r"\s*#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]\s*", raw) is not None


def inside_test_mod(node: tree_sitter.Node, source: bytes) -> bool:
    """Return True when *node* is inside a ``#[cfg(test)] mod …`` block."""
    current = node.parent

    while current is not None:
        if current.type == "mod_item":
            for attr in _leading_attributes(current, source):
                if _is_cfg_test_attr(attr, source):
                    return True

        current = current.parent

    return False


def _build_test_module_set(root: Path, rust_files: list[Path]) -> set[Path]:
    """Pre-pass: parse every Rust file to find ``#[cfg(test)] mod <name>;``
    declarations and map them to the corresponding file paths."""

    test_modules: set[Path] = set()

    for path in rust_files:
        source = path.read_bytes()
        tree = PARSER.parse(source)
        pending = [tree.root_node]

        while pending:
            node = pending.pop()

            if node.type == "mod_item":
                if _has_cfg_test_attribute(node, source):
                    name_node = node.child_by_field_name("name")

                    if name_node is not None:
                        mod_name = text(source, name_node)
                        test_modules.update(
                            _resolve_module_paths(path.parent, mod_name),
                        )

            pending.extend(reversed(node.named_children))

    return test_modules


def _has_cfg_test_attribute(node: tree_sitter.Node, source: bytes) -> bool:
    """Return True when *node* has a leading ``#[cfg(test)]`` attribute."""
    for attr in _leading_attributes(node, source):
        if _is_cfg_test_attr(attr, source):
            return True

    return False


def _resolve_module_paths(parent_dir: Path, mod_name: str) -> list[Path]:
    """Return possible file paths for a module declaration ``mod <name>;``
    in *parent_dir*."""

    return [
        parent_dir / f"{mod_name}.rs",
        parent_dir / mod_name / "mod.rs",
    ]


def is_test_module_file(path: Path, test_module_set: set[Path]) -> bool:
    """Return True when *path* belongs to a ``#[cfg(test)]`` module."""

    if path.name == "tests.rs":
        return True

    if "tests" in path.parts:
        return True

    if path in test_module_set:
        return True

    return False


# ---------------------------------------------------------------------------
# Error-type detection for let bindings
# ---------------------------------------------------------------------------

def _type_node_contains_error(type_node: tree_sitter.Node, source: bytes) -> bool:
    """Check whether a type annotation contains ``Error`` as a word segment."""
    name = text(source, type_node)

    return segments_contain_error(split_identifier(name))


def _expr_constructs_error(expr: tree_sitter.Node, source: bytes) -> bool:
    """Return True when *expr* directly constructs an Error type.

    Checks the RHS expression of a let-binding at the surface level:
    struct literals, call targets, macro invocations, bare identifiers,
    if/match branches.  Does NOT recurse deeply into nested blocks to
    avoid pathological performance on large files.
    """

    # --- direct Error-type naming patterns --------------------------------

    if expr.type == "struct_expression":
        name_node = expr.child_by_field_name("name")

        if name_node is not None and _type_node_contains_error(name_node, source):
            return True

    if expr.type == "call_expression":
        func = expr.child_by_field_name("function")

        if func is not None and _call_target_has_error(func, source):
            return True

    if expr.type == "field_expression":
        obj = expr.child_by_field_name("value")

        if obj is not None and _expr_constructs_error(obj, source):
            return True

    if expr.type == "macro_invocation":
        macro = expr.child_by_field_name("macro")

        if macro is not None and _type_node_contains_error(macro, source):
            return True

    # bare identifier (let e = SomeError;)
    if expr.type == "identifier":
        if segments_contain_error(split_identifier(text(source, expr))):
            return True

    # scoped path (let e = std::io::Error;)
    if expr.type in ("scoped_identifier", "scoped_type_identifier"):
        parts = text(source, expr).split("::")

        if any(segments_contain_error(split_identifier(p)) for p in parts):
            return True

    # --- one-level look into if / match / closure for Error constructors --

    if expr.type == "if_expression":
        for child in expr.named_children:
            if child.type == "block" and _expr_constructs_error(child, source):
                return True
            if child.type == "else_clause" and _expr_constructs_error(child, source):
                return True

    if expr.type == "match_expression":
        body = expr.child_by_field_name("body")

        if body is not None and _expr_constructs_error(body, source):
            return True

    if expr.type == "match_block":
        for child in expr.named_children:
            if child.type == "match_arm":
                value = child.child_by_field_name("value")

                if value is not None and _expr_constructs_error(value, source):
                    return True

    if expr.type == "closure_expression":
        body = expr.child_by_field_name("body")

        if body is not None and _expr_constructs_error(body, source):
            return True

    if expr.type == "block":
        for child in expr.named_children:
            if child.type == "expression_statement":
                if _expr_constructs_error(child, source):
                    return True
            # direct expression in block (last expression)
            if child.type in (
                "identifier", "scoped_identifier", "call_expression",
                "struct_expression", "field_expression",
            ):
                if _expr_constructs_error(child, source):
                    return True

    if expr.type == "else_clause":
        for child in expr.named_children:
            if _expr_constructs_error(child, source):
                return True

    if expr.type == "expression_statement":
        for child in expr.named_children:
            if _expr_constructs_error(child, source):
                return True

    return False


def _named_child_by_type(
    node: tree_sitter.Node,
    child_type: str,
) -> tree_sitter.Node | None:
    """Return the first named child of *node* with the given type, or None."""
    for child in node.named_children:
        if child.type == child_type:
            return child

    return None


def _call_target_has_error(func_node: tree_sitter.Node, source: bytes) -> bool:
    """Return True when a call expression's function target refers to an
    Error type."""
    if func_node.type in ("scoped_identifier", "scoped_type_identifier"):
        parts = text(source, func_node).split("::")

        # check all segments except the last (method / associated-fn name)
        for part in parts[:-1]:
            if segments_contain_error(split_identifier(part)):
                return True

        return False

    if func_node.type == "field_expression":
        # obj.method() — only check the object (receiver), not the method name.
        # Method names like `by_error` are look-up methods, not Error constructors.
        obj = func_node.child_by_field_name("value")

        if obj is not None:
            return _expr_constructs_error(obj, source)

        return False

    if func_node.type == "identifier":
        return segments_contain_error(split_identifier(text(source, func_node)))

    return False


def let_binding_is_error_type(let_node: tree_sitter.Node, source: bytes) -> bool:
    """Return True when *let_node* binds to an Error type.

    Checks both the explicit type annotation and the initializer expression.
    """
    type_node = let_node.child_by_field_name("type")

    if type_node is not None and _type_node_contains_error(type_node, source):
        return True

    value = let_node.child_by_field_name("value")

    if value is not None and _expr_constructs_error(value, source):
        return True

    return False


# ---------------------------------------------------------------------------
# identifier checking
# ---------------------------------------------------------------------------

# context tags passed alongside each collected name
CTX_FUNCTION       = "function"
CTX_LET            = "let"
CTX_PARAMETER      = "parameter"
CTX_CONST          = "const"
CTX_STATIC         = "static"
CTX_ENUM_VARIANT   = "enum_variant"
CTX_TYPE           = "type"
CTX_FIELD          = "field"
CTX_MACRO_FIELD    = "macro_field"


def check_identifier_name(
    name: str,
    name_node: tree_sitter.Node,
    context: str,
    is_error_type: bool,
    path: Path,
    root: Path,
    diagnostics: list[str],
) -> None:
    """Check a single identifier for forbidden word segments."""

    # --- target_ prefix (FBD009) -----------------------------------------
    # Skipped for type/field contexts — only "extension" (FBD010) is
    # checked there.

    if name.startswith("target_") and context not in (CTX_TYPE, CTX_FIELD):
        diagnostics.append(
            error_line(
                path, root, name_node,
                "FBD009",
                f"'{name}' starts with forbidden 'target_' prefix",
            ),
        )
        return

    segments = split_identifier(name)

    # Structured macro field keys are identifiers too, but their `err` form
    # follows the established tracing field convention.  Only the forbidden
    # `error` segment is checked here.
    if context == CTX_MACRO_FIELD:
        if "error" in segments:
            diagnostics.append(
                error_line(
                    path, root, name_node,
                    "FBD003",
                    f"'{name}' — 'error' is forbidden — use 'err' instead",
                ),
            )
        return

    # --- error segment → always forbidden (FBD003) -----------------------
    # Skipped for type names and field declarations — only "extension"
    # (FBD010) is checked in those contexts.

    if "error" in segments and context not in (CTX_TYPE, CTX_FIELD):
        diagnostics.append(
            error_line(
                path, root, name_node,
                "FBD003",
                f"'{name}' — 'error' is forbidden — use 'err' instead",
            ),
        )
        return

    # --- err segment → context-dependent (FBD004) ------------------------
    # Skipped for type/field contexts — only "extension" (FBD010) is
    # checked there.

    if "err" in segments and context not in (CTX_TYPE, CTX_FIELD):
        err_index = segments.index("err")
        last = len(segments) - 1

        if context == CTX_FUNCTION:
            # only _err suffix is allowed (at least 2 segments, err last)
            if err_index != last or len(segments) == 1:
                diagnostics.append(
                    error_line(
                        path, root, name_node,
                        "FBD004",
                        f"'{name}' — 'err' in function names only allowed as '_err' suffix",
                    ),
                )
                return
            # _err suffix in function name → allowed
            return

        if context in (CTX_LET, CTX_PARAMETER):
            # err_ prefix (≥2 segments, err first) allowed ONLY when NOT an Error type
            if err_index == 0 and len(segments) >= 2 and not is_error_type:
                return
            # bare err or _err suffix or is_error_type → forbidden
            msg = (
                f"'{name}' — 'err' in local variables only allowed as 'err_' prefix "
                f"on non-Error types; explicit Error instantiation is forbidden"
            )
            diagnostics.append(
                error_line(path, root, name_node, "FBD004", msg),
            )
            return

        # const, static, enum_variant — err never allowed
        diagnostics.append(
            error_line(
                path, root, name_node,
                "FBD004",
                f"'{name}' — 'err' is forbidden in this context",
            ),
        )
        return

    # --- other forbidden segments -----------------------------------------
    # "extension" (FBD010) is the only segment checked in type and field
    # contexts.  Every other forbidden segment is ignored for types/fields
    # so that type re-exports, entity structs, and enum variant types are
    # not spuriously flagged.

    for segment in segments:
        if segment in FORBIDDEN_SEGMENTS:
            if segment != "extension" and context in (CTX_TYPE, CTX_FIELD):
                return

            code, message = FORBIDDEN_SEGMENTS[segment]
            diagnostics.append(
                error_line(
                    path, root, name_node,
                    code,
                    f"'{name}' — {message}",
                ),
            )
            return


# ---------------------------------------------------------------------------
# tree walking — collect definition-site names with context
# ---------------------------------------------------------------------------

def collect_definition_names(
    node: tree_sitter.Node,
    source: bytes,
    names: list[tuple[str, tree_sitter.Node, str, bool]],
) -> None:
    """Walk *node* recursively and collect every user-defined identifier name
    together with its tree-sitter node, context tag, and Error-type flag."""

    # --- declaration `name` field ---
    if node.type in DECLARATION_KINDS:
        name_node = node.child_by_field_name("name")

        if name_node is not None:
            if node.type in ("function_item", "function_signature_item"):
                ctx = CTX_FUNCTION
            elif node.type == "const_item":
                ctx = CTX_CONST
            elif node.type == "static_item":
                ctx = CTX_STATIC
            elif node.type == "enum_variant":
                ctx = CTX_ENUM_VARIANT
            elif node.type in (
                "struct_item", "enum_item", "type_item",
                "trait_item", "union_item",
            ):
                ctx = CTX_TYPE
            elif node.type == "field_declaration":
                ctx = CTX_FIELD
            else:
                ctx = CTX_LET  # unreachable

            names.append((text(source, name_node), name_node, ctx, False))

    # --- function / closure parameter ---
    if node.type == "parameter":
        for child in node.named_children:
            if child.type == "identifier":
                names.append((text(source, child), child, CTX_PARAMETER, False))
                break

    # --- structured macro field key ---
    if node.type == "macro_invocation":
        token_tree = next(
            (
                child
                for child in node.named_children
                if child.type == "token_tree"
            ),
            None,
        )

        if token_tree is not None:
            children = token_tree.children

            for index, child in enumerate(children[:-1]):
                next_child = children[index + 1]

                if (
                    child.type == "identifier"
                    and next_child.text == b"="
                ):
                    names.append((text(source, child), child, CTX_MACRO_FIELD, False))

    # --- let binding (pattern → identifier, with Error-type detection) ---
    if node.type == "let_declaration":
        pattern = node.child_by_field_name("pattern")

        if pattern is not None:
            is_err_type = let_binding_is_error_type(node, source)
            _collect_pattern_identifiers(pattern, source, names, CTX_LET, is_err_type)

    # --- for-loop variable ---
    if node.type == "for_expression":
        pattern = node.child_by_field_name("pattern")

        if pattern is not None:
            _collect_pattern_identifiers(pattern, source, names, CTX_LET, False)

    # --- match arm binding ---
    if node.type == "match_pattern":
        _collect_pattern_identifiers(node, source, names, CTX_LET, False)

    # recurse
    for child in node.named_children:
        collect_definition_names(child, source, names)


def _collect_pattern_identifiers(
    pattern: tree_sitter.Node,
    source: bytes,
    names: list[tuple[str, tree_sitter.Node, str, bool]],
    context: str,
    is_error_type: bool,
) -> None:
    if pattern.type == "identifier":
        names.append((text(source, pattern), pattern, context, is_error_type))
        return

    if pattern.type in ("tuple_pattern", "tuple_struct_pattern"):
        for child in pattern.named_children:
            _collect_pattern_identifiers(child, source, names, context, is_error_type)
        return

    if pattern.type == "struct_pattern":
        for child in pattern.named_children:
            if child.type == "field_pattern":
                pattern_child = child.child_by_field_name("pattern")

                if pattern_child is not None:
                    _collect_pattern_identifiers(pattern_child, source, names, context, is_error_type)
                else:
                    field_name = child.child_by_field_name("name")

                    if field_name is not None:
                        names.append((text(source, field_name), field_name, context, is_error_type))
        return

    if pattern.type == "or_pattern":
        for child in pattern.named_children:
            _collect_pattern_identifiers(child, source, names, context, is_error_type)
        return

    if pattern.type in ("ref_pattern", "mutable_pattern"):
        sub = pattern.named_children[0] if pattern.named_children else None

        if sub is not None:
            _collect_pattern_identifiers(sub, source, names, context, is_error_type)
        return


def _is_repo_layer(path: Path, root: Path) -> bool:
    """Return True for files under the repository port / adapter layer,
    which is exempt from forbidden-identifier checks."""
    relative = path.relative_to(root)

    return (
        "part/repo" in str(relative)
        or "part_impl/repo" in str(relative)
    )


# ---------------------------------------------------------------------------
# per-file entry point
# ---------------------------------------------------------------------------

def check_file(
    path: Path,
    root: Path,
    test_module_set: set[Path],
) -> list[str]:
    source = production_source(path, root)
    diagnostics: list[str] = []

    # entire file is a test module — skip identifier checks
    if is_test_module_file(path, test_module_set):
        return diagnostics

    # repo layer files are exempt from forbidden-identifier checks
    if _is_repo_layer(path, root):
        return diagnostics

    names: list[tuple[str, tree_sitter.Node, str, bool]] = []

    collect_definition_names(PARSER.parse(source).root_node, source, names)

    for name, name_node, context, is_error_type in names:
        if inside_test_mod(name_node, source):
            continue

        check_identifier_name(name, name_node, context, is_error_type, path, root, diagnostics)

    return diagnostics


def check_root(root: Path, ignored_files: set[Path] | None = None) -> list[str]:
    ignored_files = ignored_files or set()
    files = [
        path
        for path in rust_files(root)
        if path.resolve() not in ignored_files
    ]
    test_module_set = _build_test_module_set(root, files)

    return [
        diagnostic
        for path in files
        for diagnostic in check_file(path, root, test_module_set)
    ]


# ---------------------------------------------------------------------------
# self-test
# ---------------------------------------------------------------------------

def self_test() -> int:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        src = root / "src"
        src.mkdir()
        fixture = src / "fixture.rs"

        # ── valid: nothing forbidden ────────────────────────────────────

        fixture.write_text(
            "fn process_input(data: &[u8]) -> Vec<u8> {\n"
            "    let processed = Vec::new();\n"
            "    let err_msg = \"something\";\n"       # err_ prefix, not Error type
            "    let prev_value = 1;\n"
            "    for byte in data {\n"
            "        match byte {\n"
            "            0 => continue,\n"
            "            _ => processed.push(*byte),\n"
            "        }\n"
            "    }\n"
            "    processed\n"
            "}\n"
            "\n"
            "fn parse_err() -> Result<(), ()> { Ok(()) }\n"  # _err suffix in fn
            "\n"
            "pub struct ConnInfo {\n"
            "    pub db_conn: String,\n"
            "}\n"
            "\n"
            "const MAX_RETRIES: u32 = 3;\n"
            "static GLOBAL_CONFIG: &str = \"\";\n"
        )

        if check_root(root):
            print("self-test: valid fixture was rejected", file=sys.stderr)
            for d in check_root(root):
                print(f"  {d}", file=sys.stderr)
            return 1

        ignored_file = src / "ignored.rs"
        ignored_file.write_text("fn ignored_result() {}\n")

        if check_root(root, {ignored_file.resolve()}):
            print("self-test: ignored file was not checked", file=sys.stderr)
            return 1

        ignore_list = root / "ignore-files.txt"
        ignore_list.write_text(
            "# Paths are relative to the check root.\n"
            "src/ignored.rs\n"
        )

        if check_root(root, resolve_ignore_files(root, [], [ignore_list])):
            print("self-test: ignore list was not applied", file=sys.stderr)
            return 1

        ignored_file.unlink()

        # ── test module is skipped ──────────────────────────────────────

        fixture.write_text(
            "#[cfg(test)]\n"
            "mod tests {\n"
            "    fn helper() {\n"
            "        let result = 1;\n"
            "        let error = \"bad\";\n"
            "        let err = SomeError;\n"
            "    }\n"
            "}\n"
        )

        if check_root(root):
            print("self-test: test module was not skipped", file=sys.stderr)
            return 1

        # ── error → always forbidden (FBD003) ───────────────────────────

        fixture.write_text(
            "fn handle_error() {}\n"               # fn name
            "fn process() {\n"
            "    let error_msg = \"\";\n"           # let binding
            "    let error = MyError;\n"            # bare error variable
            "}\n"
            "const MAX_ERROR: u32 = 0;\n"           # const
            "static ERROR_CODE: u32 = 0;\n"         # static
        )

        diagnostics = check_root(root)

        if len(diagnostics) != 5:
            print(
                f"self-test FBD003: expected 5 violations, got {len(diagnostics)}",
                file=sys.stderr,
            )
            for d in diagnostics:
                print(f"  {d}", file=sys.stderr)
            return 1

        if not all("FBD003" in d for d in diagnostics):
            print("self-test FBD003: wrong code", file=sys.stderr)
            return 1

        # ── error in structured macro field keys ────────────────────────

        fixture.write_text(
            "fn process() {\n"
            "    tracing::warn!(\n"
            "        error_variant = ?SomeError,\n"
            "        err_message = %message,\n"
            "        \"failed\",\n"
            "    );\n"
            "}\n"
        )

        diagnostics = check_root(root)

        if len(diagnostics) != 1 or "error_variant" not in diagnostics[0]:
            print(
                "self-test FBD003-macro-field: error_variant was not flagged",
                file=sys.stderr,
            )
            for diagnostic in diagnostics:
                print(f"  {diagnostic}", file=sys.stderr)
            return 1

        # ── err in fn names: only _err suffix allowed ────────────────────

        fixture.write_text(
            "fn parse_err() {}     // allowed (_err suffix)\n"
            "fn err_handler() {}   // forbidden (err_ prefix in fn)\n"
            "fn do_err() {}        // allowed (_err suffix)\n"
            "fn err() {}           // forbidden (bare err)\n"
        )

        diagnostics = check_root(root)
        # parse_err → allowed, err_handler → FBD004, do_err → allowed, err → FBD004

        if len(diagnostics) != 2:
            print(
                f"self-test FBD004-fn: expected 2 violations, got {len(diagnostics)}",
                file=sys.stderr,
            )
            for d in diagnostics:
                print(f"  {d}", file=sys.stderr)
            return 1

        flagged_names = {d.split(": FBD004: ")[1].split("'")[1] for d in diagnostics if "FBD004" in d}

        if "err_handler" not in flagged_names:
            print("self-test FBD004-fn: err_handler not flagged", file=sys.stderr)
            return 1

        if "err" not in flagged_names:
            print("self-test FBD004-fn: bare err fn not flagged", file=sys.stderr)
            return 1

        # ── err in local vars: err_ prefix + not Error type = allowed ────

        fixture.write_text(
            "fn process() {\n"
            "    let err_code: u32 = 5;\n"          # allowed (err_ prefix, not Error type)
            "    let err_msg = String::new();\n"     # allowed (err_ prefix, not Error type)
            "    let err = 42;\n"                    # forbidden (bare err)
            "    let parse_err = 42;\n"              # forbidden (_err suffix in var)
            "}\n"
        )

        diagnostics = check_root(root)
        # err → FBD004, parse_err → FBD004

        if len(diagnostics) != 2:
            print(
                f"self-test FBD004-let-basic: expected 2 violations, got {len(diagnostics)}",
                file=sys.stderr,
            )
            for d in diagnostics:
                print(f"  {d}", file=sys.stderr)
            return 1

        # ── err in local vars: Error type instantiation → always forbidden

        fixture.write_text(
            "struct SomeError;\n"
            "fn process() {\n"
            "    let err_code = SomeError;\n"       # forbidden (err_ prefix BUT Error type)
            "    let err = SomeError;\n"            # forbidden (bare err + Error type)
            "    let parse_err = SomeError;\n"      # forbidden (_err suffix + Error type)
            "}\n"
        )

        diagnostics = check_root(root)
        # err_code (Error type) → FBD004, err (Error type) → FBD004, parse_err (Error type) → FBD004

        if len(diagnostics) != 3:
            print(
                f"self-test FBD004-let-error-type: expected 3 violations, got {len(diagnostics)}",
                file=sys.stderr,
            )
            for d in diagnostics:
                print(f"  {d}", file=sys.stderr)
            return 1

        # ── err: Error type from function call ──────────────────────────

        fixture.write_text(
            "struct SomeError;\n"
            "impl SomeError { fn new() -> Self { Self } }\n"
            "fn process() {\n"
            "    let err_msg = SomeError::new();\n"    # forbidden (err_ prefix + Error type via call)
            "    let e = create_error();\n"             # FBD003 (error in fn name, not err check)
            "}\n"
            "fn create_error() -> SomeError { SomeError }\n"
        )

        diagnostics = check_root(root)
        # err_msg → FBD004 (Error type via SomeError::new())
        # create_error (fn def) → FBD003

        if len(diagnostics) != 2:
            print(
                f"self-test FBD004-call: expected 2 violations, got {len(diagnostics)}",
                file=sys.stderr,
            )
            for d in diagnostics:
                print(f"  {d}", file=sys.stderr)
            return 1

        # ── err: Error type from struct expression ──────────────────────

        fixture.write_text(
            "struct ParseError { code: u32 }\n"
            "fn process() {\n"
            "    let err_info = ParseError { code: 1 };\n"  # forbidden (err_ prefix + Error type via struct expr)
            "}\n"
        )

        diagnostics = check_root(root)
        # err_info → FBD004, ParseError → NOT checked (type name)

        if len(diagnostics) != 1:
            print(
                f"self-test FBD004-struct-expr: expected 1 violation, got {len(diagnostics)}",
                file=sys.stderr,
            )
            for d in diagnostics:
                print(f"  {d}", file=sys.stderr)
            return 1

        # ── err: Error type inside if/match/closure/field-call ──────────

        fixture.write_text(
            "struct SomeError;\n"
            "struct OtherError;\n"
            "impl SomeError { fn new() -> Self { Self } }\n"
            "impl Helper {\n"
            "    fn create_error() -> SomeError { SomeError }\n"
            "}\n"
            "fn process(cond: bool) {\n"
            "    let err_val = if cond { SomeError } else { OtherError };\n"
            "    let err_out = match cond { true => SomeError::new(), _ => OtherError };\n"
            "    let err_fn = || SomeError::new();\n"
            "}\n"
        )

        diagnostics = check_root(root)
        # err_val (err_ prefix + Error type via if) → FBD004
        # err_out (err_ prefix + Error type via match) → FBD004
        # err_fn (err_ prefix + Error type via closure) → FBD004
        # create_error fn def (in impl Helper) → FBD003

        if len(diagnostics) != 4:
            print(
                f"self-test FBD004-compound: expected 4 violations, got {len(diagnostics)}",
                file=sys.stderr,
            )
            for d in diagnostics:
                print(f"  {d}", file=sys.stderr)
            return 1

        fbd004_names = {
            d.split(": FBD004: ")[1].split("'")[1]
            for d in diagnostics if "FBD004" in d
        }

        if fbd004_names != {"err_val", "err_out", "err_fn"}:
            print(
                f"self-test FBD004-compound: wrong names flagged: {fbd004_names}",
                file=sys.stderr,
            )
            return 1

        # ── parameter with err ──────────────────────────────────────────

        fixture.write_text(
            "fn process(err_code: u32) {}\n"        # allowed (err_ prefix, not Error type)
            "fn process2(err: u32) {}\n"             # forbidden (bare err)
            "fn process3(parse_err: u32) {}\n"       # forbidden (_err suffix in param)
        )

        diagnostics = check_root(root)
        # err → FBD004, parse_err → FBD004

        if len(diagnostics) != 2:
            print(
                f"self-test FBD004-param: expected 2 violations, got {len(diagnostics)}",
                file=sys.stderr,
            )
            for d in diagnostics:
                print(f"  {d}", file=sys.stderr)
            return 1

        # ── err in const / static → forbidden ──────────────────────────

        fixture.write_text(
            "const ERR_CODE: u32 = 0;\n"
            "static GLOBAL_ERR: u32 = 0;\n"
        )

        diagnostics = check_root(root)
        # ERR_CODE → FBD004, GLOBAL_ERR → FBD004

        if len(diagnostics) != 2:
            print(
                f"self-test FBD004-other: expected 2 violations, got {len(diagnostics)}",
                file=sys.stderr,
            )
            for d in diagnostics:
                print(f"  {d}", file=sys.stderr)
            return 1

        # ── other forbidden segments ────────────────────────────────────

        fixture.write_text(
            "fn parse_result() {}\n"
            "fn compute_res() {}\n"
            "fn get_closure() {}\n"
            "fn open_connection() {}\n"
            "fn begin_txn() {}\n"
            "fn commit_tx() {}\n"
            "static target_name: &str = \"\";\n"
            "fn read_previous() {}\n"
            "fn read_prev() {}\n"
            "fn read_Previous() {}\n"
        )

        diagnostics = check_root(root)
        # 8 forbidden function names + 1 static = 9, with read_prev allowed.

        if len(diagnostics) != 9:
            print(
                f"self-test other codes: expected 9 violations, got {len(diagnostics)}",
                file=sys.stderr,
            )
            for d in diagnostics:
                print(f"  {d}", file=sys.stderr)
            return 1

        # ── FBD code coverage ───────────────────────────────────────────

        fixture.write_text(
            "fn f1(result: ()) {}\n"         # FBD001
            "fn f2(res: ()) {}\n"            # FBD002
            "fn f3(error: ()) {}\n"          # FBD003
            "fn f4(err: ()) {}\n"            # FBD004
            "fn f5(closure: ()) {}\n"        # FBD005
            "fn f6(connection: ()) {}\n"     # FBD006
            "fn f7(txn: ()) {}\n"            # FBD007
            "fn f8(tx: ()) {}\n"             # FBD008
            "static target_x: u8 = 0;\n"     # FBD009
            "fn f9(extension: ()) {}\n"      # FBD010
            "fn f10(previous_value: ()) {}\n" # FBD011
            "fn f11(PreviousValue: ()) {}\n" # FBD011 via PascalCase
            "fn f12(prev_value: ()) {}\n"    # allowed replacement
        )

        diagnostics = check_root(root)
        codes = set()

        for d in diagnostics:
            for code in (
                "FBD001", "FBD002", "FBD003", "FBD004", "FBD005",
                "FBD006", "FBD007", "FBD008", "FBD009", "FBD010", "FBD011",
            ):
                if code in d:
                    codes.add(code)

        expected_codes = {f"FBD{i:03d}" for i in range(1, 12)}

        if codes != expected_codes:
            missing = expected_codes - codes
            extra = codes - expected_codes
            print(
                f"self-test: code coverage — missing {missing}, extra {extra}",
                file=sys.stderr,
            )
            return 1

    return 0


# ---------------------------------------------------------------------------
# entry point
# ---------------------------------------------------------------------------

def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument(
        "--ignore-file",
        dest="ignore_files",
        action="append",
        type=Path,
        default=[],
        metavar="PATH",
        help="skip one Rust source file; may be repeated",
    )
    parser.add_argument(
        "--ignore-list",
        dest="ignore_lists",
        action="append",
        type=Path,
        default=[],
        metavar="PATH",
        help="read newline-delimited ignored paths; may be repeated",
    )
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    root = args.root.resolve()
    ignored_files = resolve_ignore_files(root, args.ignore_files, args.ignore_lists)
    diagnostics = check_root(root, ignored_files)

    if diagnostics:
        print("\n".join(diagnostics), file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
