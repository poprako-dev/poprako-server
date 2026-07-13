# Data struct imports

Every data domain is a public module. Import the concrete type from it and use
the type bare:

```rust
use crate::data::team::CreateTeamParams;
```

`team_data`, any other `*_data` wrapper, root data re-exports, and module
qualified uses such as `team::CreateTeamParams` are forbidden. The checker
delegates to the shared Tree-sitter rule:

```bash
uv run fmt/data-struct-qualification/check.py
uv run fmt/data-struct-qualification/check.py --fix
```
