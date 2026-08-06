# Inline defer operations

`Defer::new(...)` and `DeferBatch::new(...)` are regular operations. Construct
them directly in the consuming `.step(...)` argument; never bind them to a
local variable first.

```bash
uv run fmt/defer-oper-inline/check.py
```
