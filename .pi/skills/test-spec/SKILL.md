---
name: test-spec
description: |
  Testing conventions for poprako-r. Use whenever adding, modifying, or
  reviewing Rust tests, test helpers, #[cfg(test)] modules, or test case
  documentation.
---

# Poprako-r Test Specification

Use these rules for every Rust `#[cfg(test)] mod tests` module.

## Test case descriptions

Put all test case descriptions immediately after `mod tests {`, before any
`use` items, helper structs, helper functions, or test functions.

Format each description exactly as:

```rust
// test_name(Target::method or function_path)(positive): expected behavior
// test_name(Target::method or function_path)(negative): expected failure behavior
```

Use `positive` for successful/expected-valid behavior and `negative` for
rejection, rollback, mismatch, error propagation, or invalid-input behavior.

Do not put comments directly above a `#[test]` or `#[tokio::test]` function.
If a test needs explanation, put that explanation in the module-level test case
description list.

## Shared test logic

Extract reusable test logic whenever it is shared across multiple test
functions or modules:

- Common predicates belong in a shared `#[cfg(test)]` support module.
- Repeated aggregate builders should be local helper functions unless they are
  reused across modules.
- Repeated harness or fake dependency setup should live in a shared test harness
  module, not inside each usecase test module.

Keep helpers above the test functions and below the module-level description
list and imports.

## Import style

Test modules follow `general-conventions` import rules:

- Import concrete items.
- Do not use `use super::*`.
- Do not use wildcard imports except for explicit framework/schema exceptions.
- Keep `check-use-braces` satisfied.
