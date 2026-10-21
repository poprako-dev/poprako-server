# Pure complex boundary

`src/complex` contains pure rules and data transformations. It must not import
Orchestra, repository traits or operation descriptors, Prom operations/tasks,
or call `run_on`, `step_on`, or `proxy_on`.

The separate `layer-dependency` checker also rejects `complex -> part`
dependencies, including payload types. Shared domain payloads belong in
`model` or `value` so pure rules can consume them without importing ports.

```sh
python3 linters-extra/complex-proxy-oper-boundary/check.py --self-test
python3 linters-extra/complex-proxy-oper-boundary/check.py
```
