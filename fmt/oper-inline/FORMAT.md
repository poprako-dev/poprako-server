# Inline operations

Every `poprako_orchestra::Oper` is a one-shot operation descriptor. Construct
it directly in the consuming call argument, including `run(...)`, `step(...)`,
and `exec(...)`. Binding an oper to a local variable first is forbidden,
including `Defer` and `DeferBatch`.

```bash
uv run fmt/oper-inline/check.py
uv run fmt/oper-inline/check.py --self-test
```

exactly one reference and that reference is a direct call argument.
