---
name: rust-import-style
description: Analyzes Rust module dependency graph — detects illegal cross-layer dependencies and cyclic imports.
---

# Rust Import Style — Dependency Analyzer

A Python tool (`check-import-deps.py`) that parses `use` statements across the
entire crate and validates the **module dependency graph**.

## What it checks

### 1. Strict-ancestor dependency — the only forbidden pattern

Detects imports from a module that depend on its **own strict ancestor** —
the only forbidden dependency pattern.

> A module `<path>` is a strict ancestor of `<source>` when `source` starts
> with `path` followed by at least one additional segment.

| `source` | `target` | Allowed? |
|---|---|---|
| `crate::a::b::c` | `crate` | ❌ — strict ancestor |
| `crate::a::b::c` | `crate::a` | ❌ — strict ancestor |
| `crate::a::b::c` | `crate::a::b` | ❌ — strict ancestor |
| `crate::a::b::c` | `crate::a::b::c::x` | ✅ — descendant |
| `crate::a::b::c` | `crate::a::b::d` | ✅ — sibling |
| `crate::handler::user` | `crate::usecase::user` | ✅ — horizontal cross-layer |
| `crate::handler::team` | `crate::data` | ✅ — horizontal cross-layer |

This allows free cross-layer horizontal imports (`handler` → `usecase` →
`data` → `model`) while catching accidental parent-pulling patterns where a
submodule re-imports its own parent's types — something that should be done
via `super::`. Root module (`crate / lib.rs`) imports are also checked:
anything importing from `crate` itself is flagged unless it IS crate.

### 2. Cyclic internal module dependencies

Uses Tarjan's strongly-connected-components algorithm to detect cycles in the
module dependency graph. Any group of two or more modules that form a cycle is
reported.

### 3. Use tree expansion

The tool expands brace-grouped `use` trees into individual leaf paths before
resolving targets, so `use crate::model::{comic, team}` is analyzed as two
separate edges.

## Usage

```bash
# Scan current crate (default: cwd is crate root, src/ is source dir)
uv run .agents/skills/rust-import-style/scripts/check-import-deps.py

# Scan a specific crate root
uv run .agents/skills/rust-import-style/scripts/check-import-deps.py /path/to/crate

# Only respect crate:: / self:: / super:: paths (no implicit-crate resolution)
uv run .agents/skills/rust-import-style/scripts/check-import-deps.py --no-implicit-crate

# Scan from project root
uv run .agents/skills/rust-import-style/scripts/check-import-deps.py .
```

## Output format

```
Illegal internal module dependencies:
  src/usecase/team.rs:42: crate::model::comic -> crate::part::repo
    use: crate::part::repo::team as _;
    expanded: crate::part::repo::team

Cyclic internal module dependencies:
  cycle group 1: crate::model::post, crate::model::comment
    crate::model::post -> crate::model::comment at src/model/post.rs:18
    crate::model::comment -> crate::model::post at src/model/comment.rs:22
```

## Exit codes

| Code | Meaning |
|---|---|
| `0` | No illegal dependencies or cycles found |
| `1` | One or more issues detected |
| `2` | Source directory does not exist |

## How the parser works

1. **Module discovery**: Walks `src/` recursively for `*.rs` files, mapping each
   to its crate module path. `lib.rs` → `()`, `model/comic.rs` →
   `("model", "comic")`, `model/mod.rs` → `("model",)`.
2. **Use statement parsing**: Extracts `use ...;` statements, masking string
   literals, block comments, and raw strings to avoid false positives.
3. **Tree expansion**: Expands brace-grouped use trees into individual leaf
   paths. `use crate::a::{b::{c, d}, e}` → `crate::a::b::c`,
   `crate::a::b::d`, `crate::a::e`.
4. **Path resolution**: Resolves each leaf path to an absolute module path using
   `crate::`, `self::`, `super::`, the crate name, or implicit crate scope.
5. **Target resolution**: Finds the deepest known module that the absolute path
   resolves to.
6. **Edge validation**: Checks each `source → target` edge against the
   hierarchical dependency rule.
7. **Cycle detection**: Runs Tarjan's SCC on the full directed graph.
