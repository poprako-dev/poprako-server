#!/usr/bin/env python3

from __future__ import annotations

import json
import re
import sys
from pathlib import PurePosixPath


PROTECTED_ROOTS = {"linters", "linters-extra"}
PATCH_FILE = re.compile(
    r"^\*\*\* (?:Update|Add|Delete|Move to) File: (?P<path>.+)$",
)
TARGET_TOKEN = re.compile(
    r"(?<![A-Za-z0-9_-])(?:linters|linters-extra)(?:/|(?=$|[\s'\"]))",
)
SHELL_SEPARATOR = re.compile(r"&&|\|\||[;\n|]")
REPO_MUTATION = re.compile(
    r"\bgit\s+(?:checkout|restore|reset|clean|apply|am|pull|merge|rebase|"
    r"submodule\s+update|stash\s+(?:pop|apply))\b",
)
READ_ONLY_COMMAND = re.compile(
    r"^\s*(?:rg|grep|cat|head|tail|ls|find|pwd|stat|file|readlink|wc|test)\b"
    r"|^\s*sed\s+-n\b"
    r"|^\s*git\s+(?:status|diff|show|log|ls-files|grep|rev-parse|"
    r"check-ignore|branch|describe|submodule\s+status)\b",
)


def add_confirmation_context() -> None:
    output = {
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "additionalContext": (
                "This operation touches protected project paths: linters/ or "
                "linters-extra/. Keep the normal user confirmation prompt."
            ),
        },
    }
    print(json.dumps(output))


def protected_path(path: str) -> bool:
    normalized = path.replace("\\", "/")
    parts = PurePosixPath(normalized).parts

    return any(part in PROTECTED_ROOTS for part in parts)


def patch_touches_protected_path(command: str) -> bool:
    paths = [
        match.group("path").strip()
        for line in command.splitlines()
        if (match := PATCH_FILE.match(line)) is not None
    ]

    if paths:
        return any(protected_path(path) for path in paths)

    return bool(TARGET_TOKEN.search(command))


def shell_touches_protected_path(command: str) -> bool:
    if REPO_MUTATION.search(command):
        return True

    if not TARGET_TOKEN.search(command):
        return False

    if re.search(r"[<>`]|\$\(", command):
        return True

    segments = [segment.strip() for segment in SHELL_SEPARATOR.split(command)]

    return any(segment and not READ_ONLY_COMMAND.search(segment) for segment in segments)


def main() -> int:
    try:
        event = json.load(sys.stdin)
    except (json.JSONDecodeError, OSError):
        return 0

    tool_name = event.get("tool_name")
    tool_input = event.get("tool_input") or {}
    command = tool_input.get("command") if isinstance(tool_input, dict) else None

    if not isinstance(command, str):
        return 0

    if (
        tool_name in {"apply_patch", "Edit", "Write"}
        and patch_touches_protected_path(command)
    ):
        add_confirmation_context()
        return 0

    if tool_name == "Bash" and shell_touches_protected_path(command):
        add_confirmation_context()

    # PreToolUse only adds context. Codex still presents its normal
    # confirmation prompt later when the pending tool call needs approval.

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
