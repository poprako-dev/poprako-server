# Inline operations

Every `poprako_orchestra::Oper` is a one-shot operation descriptor. Do not bind
an operation descriptor to a local variable. Dispatch operations through the
operation-receiver extension methods `run_on(...)` and `step_on(...)` only.
Executor-receiver calls such as `repo.run(&oper)` and
`repo.step(context, &oper)` are forbidden in production code.

```bash
uv run linters-extra/oper-inline/check.py
uv run linters-extra/oper-inline/check.py --self-test
```
