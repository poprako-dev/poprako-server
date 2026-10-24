"""Reject disabled checkers and configuration the shared suite cannot apply."""

from __future__ import annotations

import argparse
from pathlib import Path
import sys
import unittest

from rust_style_lint.runner import CHECKER_NAMES, load_config
from rust_style_lint.checkers import forbidden_identifiers as identifiers


CONTEXTS = frozenset({
    "module", "function", "let", "parameter", "const", "static",
    "enum_variant", "type", "field", "macro_field",
})
SECTION_KEYS = {
    "qualified-call-site": {"enforced_std_paths", "exempt_poprako_paths", "enforced_third_party_paths"},
    "spacing-style": {"ignore_dirs"},
    "generic-where": {"exclude_files"},
    "item-layout": {"exclude_files"},
    "module-dependency": {"exclude_files"},
    "visibility-style": {"allow_public_fields", "exclude_files"},
    "forbidden-identifiers": {"exclude_filenames", "skip_module_paths", "ignore_files", "words", "prefixes"},
    "trait-use-anonymous": {"macro_markers"},
}


def validate(config: dict) -> list[str]:
    errors = []
    enabled = config.get("linters", {})
    if not isinstance(enabled, dict):
        return ["linters must be a table"]
    for name in sorted(set(enabled) - set(CHECKER_NAMES)):
        errors.append(f"unknown checker: {name}")
    for name in CHECKER_NAMES:
        if enabled.get(name) is not True:
            errors.append(f"checker must be enabled: {name}")
    for section, values in config.items():
        if section == "linters":
            continue
        if section not in SECTION_KEYS:
            errors.append(f"unknown configuration section: {section}")
            continue
        if not isinstance(values, dict):
            errors.append(f"{section} must be a table")
            continue
        for key in sorted(set(values) - SECTION_KEYS[section]):
            errors.append(f"unsupported configuration key: {section}.{key}")
        for key in SECTION_KEYS[section] - {"words", "prefixes"}:
            if key not in values:
                errors.append(f"missing configuration key: {section}.{key}")
                continue
            entries = values[key]
            if not isinstance(entries, list) or any(not isinstance(value, str) or not value for value in entries):
                errors.append(f"{section}.{key} must be a list of nonempty strings")
    for section in SECTION_KEYS:
        if section not in config:
            errors.append(f"missing configuration section: {section}")
    forbidden = config.get("forbidden-identifiers", {})
    if not isinstance(forbidden, dict):
        return errors
    for key in ("skip_module_paths", "ignore_files"):
        if forbidden.get(key) != []:
            errors.append(f"forbidden-identifiers.{key} must stay empty; hand-authored source cannot be skipped")
    for table, identity, allowed in (
        ("words", "word", {"word", "code", "replacement", "contexts", "allowed_modules"}),
        ("prefixes", "prefix", {"prefix", "code", "message", "contexts"}),
    ):
        rules = forbidden.get(table)
        if not isinstance(rules, list) or not rules:
            errors.append(f"forbidden-identifiers.{table} must be a nonempty list")
            continue
        seen = set()
        for rule in rules:
            if not isinstance(rule, dict):
                errors.append(f"{table} entries must be tables")
                continue
            label = f"{table}.{rule.get(identity, '<missing>')}"
            for key in sorted(set(rule) - allowed):
                errors.append(f"unsupported configuration key: {label}.{key}")
            for key in (identity, "code"):
                if not isinstance(rule.get(key), str) or not rule[key]:
                    errors.append(f"{label}.{key} must be a nonempty string")
            if "replacement" in rule and (not isinstance(rule["replacement"], str) or not rule["replacement"]):
                errors.append(f"{label}.replacement must be a nonempty string")
            if table == "prefixes" and (not isinstance(rule.get("message"), str) or not rule["message"]):
                errors.append(f"{label}.message must be a nonempty string")
            name = rule.get(identity)
            if isinstance(name, str):
                if name in seen:
                    errors.append(f"duplicate rule: {label}")
                seen.add(name)
            contexts = rule.get("contexts")
            if not isinstance(contexts, list) or not contexts:
                errors.append(f"{label}.contexts must be a nonempty list")
                continue
            if any(not isinstance(value, str) or value not in CONTEXTS for value in contexts):
                errors.append(f"{label}.contexts contains an unsupported context")
    return errors


def module_violations(root: Path, section: dict) -> list:
    """Apply configured module contexts omitted by the shared identifier walk."""
    root = root.resolve()
    ignored = identifiers.resolve_ignore_files(root, [Path(value) for value in section.get("ignore_files", [])], [])
    segments = identifiers.configured_segments(section)
    prefixes = identifiers.configured_prefixes(section)
    found = []
    for path in identifiers.rust_files(root, section):
        if path.resolve() in ignored or identifiers.is_skipped_module_path(path, root, section):
            continue
        source = identifiers.production_source(path, root)
        tree = identifiers.PARSER.parse(source)
        pending = [(tree.root_node, identifiers.file_module(path, root))]
        while pending:
            node, module = pending.pop()
            if node.type == "mod_item":
                name = node.child_by_field_name("name")
                if name is not None:
                    spelling = source[name.start_byte:name.end_byte].decode()
                    module = (*module, spelling)
                    if not identifiers.inside_test_mod(name, source):
                        identifiers.check_identifier_name(
                            spelling, name, "module", False, path, root, found,
                            identifiers.format_module(module), segments, prefixes, {},
                        )
            pending.extend((child, module) for child in reversed(node.named_children))
    return found


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        suite = unittest.defaultTestLoader.discover(str(Path(__file__).parent), pattern="test_check.py")
        return int(not unittest.TextTestRunner().run(suite).wasSuccessful())
    try:
        config = load_config(args.root)
        errors = validate(config)
    except (OSError, ValueError) as error:
        errors = [str(error)]
    for error in errors:
        print(f"rust-style-lint.toml:1: CFG001: {error}", file=sys.stderr)
    if errors:
        return 1
    found = module_violations(args.root, config["forbidden-identifiers"])
    for violation in found:
        print(f"{violation.path}:{violation.line}: {violation.code}: {violation.message}", file=sys.stderr)
    return int(bool(found))


if __name__ == "__main__":
    raise SystemExit(main())
