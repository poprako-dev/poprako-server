# Complex port boundary

`complex` may access ports only through `Proxy<Oper>` and `proxy.exec(...)`.
It may import operation descriptors from `crate::part::repo::oper` and the
payload type required by a deferred operation, but must not import or name
direct repository or prom traits, nor call `run` or `step` on a concrete port.

```bash
uv run fmt/complex-proxy-oper-boundary/check.py
```
