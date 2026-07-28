# Model struct imports

Every model domain is a public module. Import the concrete type from it and
use the type bare:

```rust
use crate::model::shared::user::UserToken;
```

`user_model`, any other `*_model` wrapper, root model re-exports, and module
qualified uses such as `user::UserToken` are forbidden. The checker delegates
to the shared Tree-sitter rule:

```bash
uv run fmt/model-struct-qualification/check.py
uv run fmt/model-struct-qualification/check.py --fix
```
