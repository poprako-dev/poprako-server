# Data struct qualification

Outside `src/data/` and `src/model/`, a data-layer type must be referred to
through its public domain alias:

```rust
use crate::data::user_data;

fn current_user() -> user_data::InfoVal { /* ... */ }
```

Do not import individual data types, and do not use the private
`crate::data::user` module path. The domain alias makes the origin of generic
names such as `Form`, `InfoVal`, and `ListInfosData` explicit.

Run the standalone checker from the repository root:

```bash
uv run fmt/data-struct-qualification/check.py
```

The script contains its pinned Tree-sitter dependencies, so `uv run` is the
only preparation it needs. It reports violations only and never modifies a
Rust file.
