# Model struct naming

Model structs must not repeat the name of their domain module. The public
`*_model` module already supplies that qualification:

```rust
use crate::model::team_model;

fn create(form: team_model::Form) { /* ... */ }
```

Write `Info`, `Form`, or `ListSpec`, not `TeamInfo`, `TeamForm`, or
`TeamListSpec` in `src/model/team.rs`.

Run the standalone checker from the repository root:

```bash
uv run fmt/model-struct-naming/check.py
```

The script contains its pinned Tree-sitter dependencies, reports violations
only, and never modifies a Rust file.
