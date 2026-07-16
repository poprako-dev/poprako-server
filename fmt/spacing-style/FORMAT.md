# Rust block spacing

`just fmt-check` runs this checker. It validates the direct statements in Rust
blocks and direct match arms:

- `BLK000`: a multi-statement block whose opening `{` shares a line with its
  header must start with a bare `//` separator;
- `BLK001`: direct statements and match arms must have a blank line between
  them; and
- `BLK002`: a single-statement block must not start with a bare `//`
  separator.

Run it directly with:

```bash
fmt/.venv/bin/python fmt/spacing-style/check.py
fmt/.venv/bin/python fmt/spacing-style/check.py --fix
```
