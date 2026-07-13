# Direct struct imports

`model` and `data` domains are public modules. Import every concrete type from
its domain module and use it bare: `use crate::model::user::UserToken;` then
`UserToken`. `*_model`, `*_data`, flat layer re-exports, and qualified domain
type paths in Rust bodies are forbidden.

The checker is pure Tree-sitter traversal; it never classifies Rust paths with
regular expressions.

```bash
uv run fmt/direct-struct-import/check.py --layer model
uv run fmt/direct-struct-import/check.py --layer data
```

`--fix` removes whole legacy imports that contain only `*_model` or
`*_data` module aliases. It never rewrites a mixed import or a type use.
