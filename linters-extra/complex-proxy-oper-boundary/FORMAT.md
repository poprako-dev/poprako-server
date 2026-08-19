# Pure complex boundary

`src/complex` contains pure rules and data transformations. It must not import
Orchestra, repository traits or operation descriptors, Prom operations/tasks,
or call `run_on`, `step_on`, or `proxy_on`.

Pure payload/value types remain allowed, including
`part::prom::payload::image::ResourceKind`.

```sh
python3 linters-extra/complex-proxy-oper-boundary/check.py --self-test
python3 linters-extra/complex-proxy-oper-boundary/check.py
```
