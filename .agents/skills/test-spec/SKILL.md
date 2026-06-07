---
name: test-spec
description: "Test module conventions: use super::* first, test-case descriptions before imports, format // name(target)(positive|negative): desc, no comments above #[test]."
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

Test modules must use `use super::*` to bring all parent-module items into
scope. This is the only place where wildcard imports (`*`) are permitted —
outside of test modules, `use super::*` and any other wildcard import are
forbidden.

- Always use `use super::*` at the top of each `#[cfg(test)] mod tests` block.
- Import additional concrete items below `use super::*` as needed.
- Do not use wildcard imports except for `use super::*` in tests and explicit
  framework/schema exceptions.
- Keep `check-use-braces` satisfied.
