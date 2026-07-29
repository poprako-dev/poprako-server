# Trait import anonymisation

Traits imported only to enable method resolution must be imported anonymously:

```rust
use poprako_util::time::ToUnixMilli as _;

let timestamp = value.to_unix_milli();
```

When the trait name is explicitly used in a bound, `impl Trait`, `dyn Trait`,
qualified call, or derive attribute, the import remains named because it is
part of the source-level type or macro expression:

```rust
use crate::part::repo::team::TeamRepo;

fn list<R: TeamRepo<Context>>() { /* ... */ }
```

The checker parses Rust ASTs, collects trait definitions from the workspace
source, and checks imported trait names against later AST identifiers. It also
recognises the external trait paths covered by the project's import rules.
It reports method-resolution-only imports and never rewrites Rust files.

```bash
uv run fmt/trait-use-anonymous/check.py
uv run fmt/trait-use-anonymous/check.py --self-test
```
