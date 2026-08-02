# Rust block spacing

`sh scripts/ci-check.sh` runs this checker. It validates direct statements in Rust
blocks, direct match arms, enum variants, and multi-field structs:

- `BLK000`: a multi-statement block or multi-field struct whose opening `{`
  shares a line with its header must start with a bare `//` separator;
- `BLK001`: direct statements, match arms, and enum variants must have a blank
  line between them; and
- `BLK002`: a single-statement block or single-field struct must not start with a bare `//`
  separator.

Run it directly with:

```bash
fmt/.venv/bin/python fmt/spacing-style/check.py
fmt/.venv/bin/python fmt/spacing-style/check.py --fix
```
