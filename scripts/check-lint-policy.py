#!/usr/bin/env python3
"""Verify that every workspace target inherits the strict lint policy."""

import json
from pathlib import Path
import subprocess
import sys
import tomllib


REQUIRED_CLIPPY = {
    "all", "pedantic", "nursery", "unwrap_used", "expect_used", "panic",
    "unreachable", "todo", "unimplemented", "dbg_macro", "print_stdout",
    "print_stderr", "exit", "indexing_slicing", "string_slice",
    "mod_module_files", "allow_attributes_without_reason",
}


def level(setting):
    if isinstance(setting, str):
        return setting

    if isinstance(setting, dict):
        return setting.get("level")

    return None


def inspect_policy(metadata):
    root = Path(metadata["workspace_root"])
    manifest = tomllib.loads((root / "Cargo.toml").read_text())
    policy = manifest.get("workspace", {}).get("lints", {})
    errors = []

    for name, expected in {"warnings": "deny", "unsafe_code": "forbid"}.items():
        if level(policy.get("rust", {}).get(name)) != expected:
            errors.append(f"Cargo.toml: rust::{name} must be {expected}")

    for name in sorted(REQUIRED_CLIPPY):
        if level(policy.get("clippy", {}).get(name)) not in {"deny", "forbid"}:
            errors.append(f"Cargo.toml: clippy::{name} must be deny or forbid")

    for group in ("all", "pedantic", "nursery"):
        setting = policy.get("clippy", {}).get(group)

        if not isinstance(setting, dict) or setting.get("priority") != -1:
            errors.append(f"Cargo.toml: clippy::{group} must have priority -1")

    for tool, rules in policy.items():
        for name, setting in rules.items():
            if level(setting) not in {"deny", "forbid"}:
                errors.append(f"Cargo.toml: workspace {tool}::{name} weakens the policy")

    members = set(metadata["workspace_members"])

    for package in metadata["packages"]:
        if package["id"] not in members:
            continue

        path = Path(package["manifest_path"])
        package_manifest = tomllib.loads(path.read_text())

        if package_manifest.get("lints") != {"workspace": True}:
            errors.append(f"{path}: must exclusively inherit workspace lints")

    return errors


def main():
    root = Path(__file__).resolve().parent.parent
    result = subprocess.run(
        ["cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    )
    errors = inspect_policy(json.loads(result.stdout))

    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1

    print("Workspace lint policy and member inheritance passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
