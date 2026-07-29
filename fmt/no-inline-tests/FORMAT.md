# No inline test modules

`#[cfg(test)] mod tests { ... }` must be written as a separate file reference
`#[cfg(test)] mod tests;` — the test code lives in a dedicated `tests.rs` or
`tests/` sibling file. Inline test modules clutter production files and defeat
the module-per-file convention.

```rust
// ❌ Forbidden — inline body
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn something_works() { }
}

// ✅ Required — separate file
#[cfg(test)]
mod tests;
```

The checker scans every `*.rs` file under `src/` and reports `TST001` for each
inline `mod tests { ... }` with `#[cfg(test)]`.

## Checker

```bash
uv run fmt/no-inline-tests/check.py --self-test
uv run fmt/no-inline-tests/check.py
```
