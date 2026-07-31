# Forbidden identifiers

Function names, local variables, parameters, constants, statics, enum
variants, type names (struct, enum, trait, type alias, union), and struct
fields must not contain forbidden word segments. Structured macro field keys
are checked for the forbidden `error` segment as well.

Identifiers inside `#[cfg(test)]` modules (including `tests.rs` and `tests/`
directories) and repository-layer files (`src/part/repo/`,
`src/part_impl/repo/`) are skipped entirely.

## Forbidden word segments

Each identifier is split into word segments by snake_case and PascalCase
boundaries. Every segment is checked:

| Segment      | Code   | Rule                                              |
| ------------ | ------ | ------------------------------------------------- |
| `result`     | FBD001 | forbidden — name what the value represents        |
| `res`        | FBD002 | forbidden abbreviation of `result`                |
| `error`      | FBD003 | always forbidden — use `err`                      |
| `closure`    | FBD005 | forbidden word                                    |
| `connection` | FBD006 | forbidden — use `conn`                            |
| `txn`        | FBD007 | forbidden abbreviation of `transaction`           |
| `tx`         | FBD008 | forbidden abbreviation of `transaction`           |
| `extension`  | FBD010 | forbidden — use `ext`                             |
| `previous`   | FBD011 | forbidden — use `prev`                            |

## `err` rules (FBD004)

`err` is the **replacement** for `error`, but its form is restricted:

| Context              | Allowed form     | Example (OK)        | Forbidden                        |
| -------------------- | ---------------- | ------------------- | -------------------------------- |
| Function names       | `_err` suffix    | `fn parse_err()`    | `fn err_handler()`, `fn err()`   |
| Local vars/params    | `err_` prefix    | `let err_msg = "x"` | bare `err`, `_err` suffix        |
|                      | **and** NOT Error type |                  | any `err` with Error type        |
| const, static        | *never*          | —                   | any `err` form                   |

A local variable with `err_` prefix that IS an Error type (detected via type
annotation, `SomeError::new()`, `ParseError { … }`, or bare `SomeError`
identifier) is always forbidden.

## Other patterns

- **FBD009** — identifiers starting with `target_` are forbidden.
- **FBD010** — `schema::` path qualifier (outside `rdb_impl/`) is forbidden.
- **FBD011** — `previous` is forbidden; use `prev` instead.

## Examples

```rust
// ✗ FBD003 — error always forbidden
fn handle_error() {}
let error_msg = "";
const MAX_ERROR: u32 = 0;

// ✓ FBD003 fix — replace error with err
fn handle_err() {}

// ✗ FBD004 — err in fn must be _err suffix
fn err_handler() {}
fn err() {}

// ✓ FBD004 fix — _err suffix in fn
fn parse_err() {}
fn handle_err() {}

// ✗ FBD004 — local var: bare err or _err suffix forbidden
let err = 42;
let parse_err = 42;

// ✓ FBD004 fix — err_ prefix on non-Error type
let err_code: u32 = 5;
let err_msg = String::new();

// ✗ FBD004 — err_ prefix BUT Error type
let err_code = SomeError::new();
let err_info = ParseError { code: 1 };

// ✗ FBD001 / FBD002 / FBD005 / FBD006 / FBD007 / FBD008 / FBD011
fn parse_result() {}
fn compute_res() {}
fn get_closure() {}
fn open_connection() {}
fn begin_txn() {}
fn commit_tx() {}
fn read_previous() {}

// ✓ FBD011 fix — replace `previous` with `prev`
fn read_prev() {}
```

## Usage

```bash
uv run fmt/forbidden-identifiers/check.py
uv run fmt/forbidden-identifiers/check.py --self-test
```

The script scans every `.rs` file under `src/` (excluding `schema.rs` and
`#[cfg(test)]` modules), reports violations to stderr, and exits with 0
(clean) or 1 (violations found). There is no `--fix` mode.
