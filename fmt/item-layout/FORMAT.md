# Rust item layout

Hand-written production Rust under `src/` follows these declaration-ordering
rules:

1. When a struct has implementations in the same file and module, those impl
   blocks directly follow the struct. Its inherent `impl Type` block comes
   before every `impl Trait for Type` block.
2. In an `impl` block or module file, public functions come before private
   helper functions.
3. Private helpers are ordered by the first call to each helper. Helpers never
   called in that scope remain after called helpers and preserve their existing
   relative order.

The checker scans source under `src/`. Test-only `#[cfg(test)]` items and the
generated Diesel schema are excluded. It supports automatic repair by moving
complete declarations, including directly attached doc comments and
attributes. It never rewrites a function body:

```bash
fmt/.venv/bin/python fmt/item-layout/check.py
fmt/.venv/bin/python fmt/item-layout/check.py --fix
fmt/.venv/bin/python fmt/item-layout/check.py --self-test
```

Diagnostic codes:

- `LAYOUT001`: a struct impl is separated from its declaration;
- `LAYOUT002`: an inherent impl follows a trait impl;
- `LAYOUT003`: a private function precedes a public function; and
- `LAYOUT004`: private helpers are not in first-call order.
