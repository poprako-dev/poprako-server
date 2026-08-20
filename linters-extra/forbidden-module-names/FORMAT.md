# Forbidden Rust module names

Rust module declarations under `src/` must describe the business or technical
responsibility they own. Generic container names hide that responsibility.

The following module names are forbidden:

- `helper` and `helpers` (`MODNAME001`)
- `operation` and `operations` (`MODNAME002`)

The checker examines every `mod` declaration, including test-only modules.
Generated Diesel `schema.rs` files are excluded.

```sh
python3 linters-extra/forbidden-module-names/check.py --self-test
python3 linters-extra/forbidden-module-names/check.py
```
