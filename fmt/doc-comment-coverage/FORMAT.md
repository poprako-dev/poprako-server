# Identifier comment coverage

Every user-defined identifier must have a preceding comment that matches its
visibility:

- Public identifiers require an outer documentation comment (`///` or `/**`).
  This keeps the public API discoverable through `cargo doc`.
- Private identifiers require an ordinary comment (`//` or `/*`). This records
  implementation intent without exposing it as public documentation.

## What is checked

| Declaration kind         | Scope                          |
| ------------------------ | ------------------------------ |
| `mod`                    | module-level module declaration |
| `fn`                     | module-level and impl methods  |
| `struct`                 | module-level                   |
| `enum`                   | module-level                   |
| `trait`                  | module-level                   |
| `type`                   | module-level type alias        |
| `const`                  | module-level                   |
| `static`                 | module-level                   |
| `macro`                  | module-level macro definition  |
| `union`                  | module-level                   |
| struct / union field     | every named field              |
| trait `fn`               | trait method signatures        |
| trait `type`             | trait associated types         |
| trait `const`            | trait associated constants     |
| `enum` variant           | every variant of every enum    |
| `pub fn` in `impl` block | inherent public methods        |

## What is skipped

- Items annotated with `#[test]`, `#[tokio::test]`, or `#[rstest…]`.
- Source files under `tests/`, `entity/`, or `oper/` path segments, and
  files whose path contains a `test_*` segment — these are test fixtures,
  Diesel entity directories, operation descriptors (self-documenting by
  naming convention), or test helpers.
- Files named `entity.rs` — Diesel entity definitions.
- The `main` function in `src/main.rs`.
- Diesel-generated `schema.rs`.
- Inner doc comments (`//!`, `/*!`) — they document the enclosing module,
  not the following item.

## Running the checker

```bash
# Check the full workspace
uv run fmt/doc-comment-coverage/check.py

# Self-test
uv run fmt/doc-comment-coverage/check.py --self-test
```

The script reports violations only and never modifies any Rust file.
Each violation is reported in the format:

```
path/to/file.rs:42: DOC001: private function 'do_thing' is missing a regular comment
```
