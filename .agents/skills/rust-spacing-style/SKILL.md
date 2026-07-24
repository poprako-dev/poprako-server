---
name: rust-spacing-style
description: Checks for redundant empty `//` separator comments in single-statement Rust blocks. Detects BFX001.
---

# Rust Spacing Style

A custom Python parser (`check-spacing-style.py`) that checks for **redundant
empty `//` separator comments** inserted before the first statement of a
single-statement block.

## Rule: BFX001 — Redundant block-start separator in single-statement block

When a block (`fn`, `if`, `else`, `for`, `while`, `match arm`, etc.) contains
**exactly one statement**, any `//` comment lines or blank lines between the
`{` and that statement are redundant and should be removed.

```rust
// ❌ BFX001 — single-statement block with unnecessary `//` separator
fn gen_id() -> String {
    //
    next_snowflake_id()
}

// ✅ OK — single-statement block, no separator
fn gen_id() -> String {
    next_snowflake_id()
}

// ✅ OK — two or more statements, separators are allowed
fn resolve_id(id: &str) -> Result<Id> {
    //
    let parsed = parse(id)?;

    validate(&parsed)
}
```

### Related spacing rules

The checker also enforces a bare `//` after a non-standalone opening `{` in
multi-statement blocks and multi-field structs, and one blank line between
adjacent direct statements, match arms, and enum variants.

### Why

Single-statement blocks are simple enough that a `//` separator adds visual
clutter without providing structure. The purpose of a start-comment is to
separate the block header from the body — when there's only one statement,
that separation is unnecessary.

### Exempt blocks

- **Item bodies** (`impl`, `trait`, `mod`, `extern`) — their `{` is typically
  on its own line (Allman style), so the rule does not apply.
- **Literal blocks** (struct patterns, macro_rules definitions) — non-code.
- **Root block** (file-level scope).

### With `--fix`

The script can automatically remove the redundant separator lines:

```bash
uv run .agents/skills/rust-spacing-style/check-spacing-style.py src/ --fix
```

Both empty `//` comment lines and adjacent blank lines in the separator
region are removed.

## Block classification

The parser classifies each `{ ... }` block to decide whether the separator
check applies:

| Kind | check_start_separator? | Examples |
|---|---|---|
| `root` | No | File-level scope |
| `fn_body` | Yes | Function bodies |
| `item_body` | No | `impl`, `trait`, `mod`, `extern` bodies |
| `control_body` | Yes | `if`, `else`, `for`, `while`, `loop`, `match`, `unsafe` bodies |
| `closure_body` | Yes | Closure `\|...\| { ... }` and `async move { ... }` bodies |
| `match_arm_body` | Yes | Match arm `=> { ... }` bodies |
| `block_expr` | Yes | Stand-alone `{ ... }` expression blocks |
| `literal` | No | Non-code `{ ... }` (struct patterns, macro_rules bodies, etc.) |

## How the parser works

1. **Sanitize**: Replace string literals (`"..."`, `'...'`), comments
   (`//...`, `/*...*/`), and raw strings (`r#"..."#`, etc.) with whitespace,
   preserving line structure.
2. **Parse into blocks**: Walk the sanitized source character by character.
   Track `( )`, `[ ]` depth. On `{`, classify the block (checks the preceding
   prefix for keywords like `fn`, `match`, `if`, or previous token like `=>`).
   On `}`, pop the block. Literal blocks (struct patterns) are transparent.
3. **Collect statements**: A statement starts on any non-whitespace character
   and ends at `;` or when a child block closes.
4. **Run BFX001**: For each code block with `check_start_separator = True`
   that has exactly one statement: check the lines between the `{` line and
   the first statement. If only `//` comments and blank lines are found,
   emit a BFX001 diagnostic.

## Usage

```bash
# Scan and print diagnostics
uv run .agents/skills/rust-spacing-style/check-spacing-style.py

# Scan and auto-fix
uv run .agents/skills/rust-spacing-style/check-spacing-style.py src/ --fix

# Scan a specific file
uv run .agents/skills/rust-spacing-style/check-spacing-style.py src/complex/unit.rs
```

## Output format

```
<file>:<line>:<col>: <CODE>: <message>
```

| Code | Meaning |
|---|---|
| `BFX001` | Redundant `//` separator comment in a single-statement block |

The script exits with:
- `0` — no issues found
- `1` — one or more diagnostics reported (or fixes applied)
- `2` — file read error

When `--fix` is used, a summary line `fixed N redundant separator block(s)`
is printed at the end.
