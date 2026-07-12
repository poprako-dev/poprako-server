# Data struct naming

Data structs must not repeat the name of their domain module. The public
`*_data` module already supplies that qualification:

```rust
use crate::data::team_data;

fn create(data: team_data::CreateData) { /* ... */ }
```

In `src/data/team.rs`, write `CreateData` rather than `TeamCreateData`.
Every data struct represents either an inbound request or an outbound value:

- Request structs end in `Data`.
- Response structs, including structs placed in a response's `val`, end in
  `Val`.

Run the standalone checker from the repository root:

```bash
uv run fmt/data-struct-naming/check.py
```

The script contains its pinned Tree-sitter dependencies, reports violations
only, and never modifies a Rust file.
