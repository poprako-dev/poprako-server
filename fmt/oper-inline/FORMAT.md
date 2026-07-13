# Inline operations

Every `poprako_orchestra::Oper` is a one-shot operation descriptor. Construct
it directly in the consuming `run(...)` or `step(...)` argument. Binding an
oper to a local variable first is forbidden, including `Defer` and
`DeferBatch`.

```bash
uv run fmt/oper-inline/check.py
```

`--fix-safe` only rewrites a local oper when its next statement contains
exactly one reference and that reference is a direct call argument.
