# Inline defer operations

`Defer::new(...)` and `DeferBatch::new(...)` must be constructed directly as
the receiver of `.step_on(...)`. They may not be bound to a local variable,
passed as an argument, or dispatched through any other method.

```bash
uv run linters-extra/defer-oper-inline/check.py
uv run linters-extra/defer-oper-inline/check.py --self-test
```
