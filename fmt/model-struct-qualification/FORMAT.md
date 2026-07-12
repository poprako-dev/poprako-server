# Model struct qualification

Outside `src/data/` and `src/model/`, a model-layer type must be referred to
through its public domain alias:

```rust
use crate::model::team_model;

fn create(form: team_model::Form) { /* ... */ }
```

Do not import individual model types, and do not use the private
`crate::model::team` module path. The domain alias keeps generic model names
such as `Info`, `Form`, and `ListSpec` unambiguous.

Run the standalone checker from the repository root:

```bash
uv run fmt/model-struct-qualification/check.py
```

The script contains its pinned Tree-sitter dependencies, so `uv run` is the
only preparation it needs. It reports violations only and never modifies a
Rust file.
