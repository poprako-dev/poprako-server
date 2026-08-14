# Complex port boundary

`complex` may access ports only through `Proxy<Oper>` and `proxy.exec(...)`.
It may import operation descriptors from `crate::part::repo::oper` and
`crate::part::prom::oper`, and the payload or task data required by a
deferred operation from `crate::part::prom::payload` and
`crate::part::prom::task`, but must not import or name direct repository or
prom traits, nor call `run` or `step` on a concrete port.

```bash
uv run fmt/complex-proxy-oper-boundary/check.py
```
