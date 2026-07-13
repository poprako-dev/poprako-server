# Model struct naming

Public model types must carry their domain in the type name. Their meaning must
not depend on a `*_model::` path prefix:

```rust
use crate::model::TeamEntry;

fn create(entry: TeamEntry) { /* ... */ }
```

In `src/model/team.rs`, write `TeamInfo`, `TeamEntry`, and `TeamListSpec`, not
the bare `Info`, `Form`, or `ListSpec` names. `Form` is forbidden: a model used
as the row input for creation is an `Entry`; updates and reservations use an
equally precise domain-qualified role name.

Run the standalone checker from the repository root:

```bash
uv run fmt/model-struct-naming/check.py
uv run fmt/model-struct-naming/check.py --self-test
```

The script contains its pinned Tree-sitter dependencies, reports violations
only, and never modifies a Rust file.
