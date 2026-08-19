# Orchestra operation proxy ban

Rust source must not use operation proxy traits, generated repository/Prom
proxy traits, proxy macros, `.proxy_on`, or `#[drive(proxy = ...)]`.

`NuclProxy` is a composition type and is explicitly exempt.

```sh
python3 linters-extra/orchestra-proxy-ban/check.py --self-test
python3 linters-extra/orchestra-proxy-ban/check.py
```
