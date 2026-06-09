---
name: test-spec
description: "Test module conventions: use super::* first, test-case descriptions before imports, format // name(target)(positive|negative): desc, no comments above #[test], ≥1 positive + ≥1 negative per pub usecase function, more tests for complex functions."
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

## Usecase test-case minimums

Every `pub` function in `src/usecase/` must have **at least** the following
number of test cases in the corresponding `#[cfg(test)] mod tests` block:

| Category  | Minimum | Purpose                                         |
|-----------|---------|-------------------------------------------------|
| positive  | 1       | Verify the happy path succeeds without error.   |
| negative  | 1       | Verify at least one failure propagates correctly. |

These are **minimums** — every `pub` usecase function must have at least one
positive and one negative test, even if the function is a thin wrapper around
a single query.

### When to add more tests

**More positive tests** are required when:

- The function accepts parameters that produce **observably different
  outcomes** in the response or side effects (e.g., `list` with keyword vs.
  without, different `RoleFlag` filters, pagination edge cases).
- The function has **multiple semantically distinct happy-path branches**
  (e.g., `reserve_avatar` producing a delete-message vs. not when a previous
  avatar exists).

**More negative tests** are required **for each genuinely distinct error
path** the function can traverse:

- Not-found error vs. conflict error vs. authentication error vs.
  unrecoverable error — each counts as one distinct path.
- Do **not** write multiple negative tests that all hit the same error
  variant with only trivial input variations.  If the function only has one
  error path (e.g., "not found"), one negative test is sufficient.

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
