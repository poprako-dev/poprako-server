# Doc comment coverage

Every public user-defined identifier must carry an outer documentation
comment (`///` or `/**`). This makes the public API surface discoverable
through `cargo doc` and helps reviewers understand the intent of each
exported item without reading the implementation.

## What is checked

| Declaration kind         | Scope                          |
| ------------------------ | ------------------------------ |
| `pub mod`                | module-level module declaration |
| `pub fn`                 | module-level and impl methods  |
| `pub struct`             | module-level                   |
| `pub enum`               | module-level                   |
| `pub trait`              | module-level                   |
| `pub type`               | module-level type alias        |
| `pub const`              | module-level                   |
| `pub static`             | module-level                   |
| `pub macro`              | module-level macro definition  |
| `pub union`              | module-level                   |
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
- (none — struct / enum / union fields are now checked)
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
path/to/file.rs:42: DOC001: public function 'do_thing' is missing a doc comment
```
