# Data struct naming

Public data types must carry their domain in the type name. Their meaning must
not depend on a `*_data::` path prefix:

```rust
use crate::data::CreateTeamParams;

fn create(params: CreateTeamParams) { /* ... */ }
```

In `src/data/team.rs`, write `CreateTeamParams`, not `CreateData`. Every public
data type has one boundary role:

- Usecase and handler inputs end in `Params`.
- Usecase return DTOs end in `Payload`.
- `Val` is reserved for a serde representation converted from a model, such as
  `TeamInfo -> TeamInfoVal`; it is not a generic response suffix.

`Form` and the legacy `Data` suffix are forbidden. A persisted creation input
belongs in the model layer as a domain-qualified `Entry`.

Run the standalone checker from the repository root:

```bash
uv run fmt/data-struct-naming/check.py
uv run fmt/data-struct-naming/check.py --self-test
```

The script contains its pinned Tree-sitter dependencies, reports violations
only, and never modifies a Rust file.
