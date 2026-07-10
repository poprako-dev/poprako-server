---
name: general-conventions
description: Project-wide Rust conventions for active PopRaKo modules. Use whenever creating, moving, refactoring, or reviewing Rust code under src/.
---

# Active Rust Conventions

## Scope

Apply this skill to the active module tree: `api`, `complex`, `data`, `harn`, `model`, `part`, `part_impl`, `result`, `usecase`, and `value`. Do not follow examples or paths from removed `domain`, `infra`, query, or test-harness architectures.

## Comments and module shape

- Keep source and doc comments in English and describe current Rust behavior.
- Read the nearby module before selecting names, visibility, imports, or tests.
- Keep a Rust source file below 600 lines; extract a focused module before it becomes larger.
- Prefer `pub` for a deliberate public contract. Keep implementation details private rather than relying on broad crate-scoped visibility.

## Imports, names, and construction

- Group standard-library, third-party, workspace, and crate imports as nearby files do. Merge matching prefixes without non-leaf brace imports.
- Use full, domain-specific local names: `comic_info`, `workset_form`, `cover_reservation`, and `system_mail_infos`.
- Bind a form, update, spec, or DTO before passing it to a step factory. Do not embed an inline struct literal in a step call.
- Follow current public type names. Request DTOs end in `Data`, response DTOs end in `Val`, and repository operations are described by step types.

## Control flow and spacing

- Do not write `if ... else`. Use `match`, guards, early returns, or `let ... else`.
- Leave a blank line between statements and logical groups of struct fields.
- Remove unused parameters when the signature is controlled locally; use bare `_` only when a required trait signature forces it.

## Trait calls

Use the local project's established trait-call style. Repository operations are normally called through `repo.execute(...)` or `repo.advance(...)`; use UFCS only when it clarifies an ambiguous implementation or the nearby code does so.

## Checklist

- [ ] The code uses active module paths and current names.
- [ ] Typed locals, fields, and parameters are specific.
- [ ] No inline step struct literal or `if ... else` was added.
- [ ] Comments and public docs describe active behavior.
