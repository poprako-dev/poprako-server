# Use-case module aliases

Import use-case operation modules with descriptive snake_case `*_usecase`
aliases, then reference their functions through those aliases. The caller names
the module that directly owns the operation, rather than navigating from the
`usecase` layer root or an ancestor module.

```rust
use crate::usecase::user as user_usecase;
use crate::usecase::user::delete as user_delete_usecase;
use crate::usecase::chapter::stage as chapter_stage_usecase;

async fn run() {
    user_usecase::update(/* arguments */).await;
    user_delete_usecase::delete(/* arguments */).await;
    chapter_stage_usecase::update(/* arguments */).await;
}
```

Do not import use-case functions directly, rename individual functions, or
import all functions with a glob. Do not use `usecase::user::update()` or
`user_usecase::delete::delete()`; import the directly owning module instead.
Function pointers follow the same convention as calls, including generic
function references.

Use descriptive names such as `user_usecase`, `user_delete_usecase`, and
`chapter_stage_usecase`. The checker validates the snake_case suffix pattern;
domain-specific spelling remains a review convention.

Data types can be imported directly. Local calls within the defining module
need no alias. Presentation assembly modules named `view` and modules behind a
private module boundary are helper implementations, so this checker exempts
them. For example, `usecase::internal`, private invitation `code`, and the
private root `usecase::stage` are exempt, while the public
`usecase::chapter::stage` remains checked.

## Checker

The checker discovers public free functions through the public module graph
rooted at `src/usecase.rs`, including inline modules, and scans production Rust
under `src/`. It uses the shared production-source masker to exclude tests and
mocks. Existing CI discovers this directory automatically.

- `UCA001`: direct or glob import of public use-case functions.
- `UCA002`: import of an operation module without a snake_case `*_usecase` alias.
- `UCA003`: qualified function reference that does not use the directly owning
  imported module as `*_usecase::function`.

Resolution is syntax-based, not a full Rust name resolver. It handles nested
use trees, ordinary qualified paths, and lexical import scopes. It does not
expand macros, infer types, or follow arbitrary re-export chains. Re-export
restrictions are checked separately by `qualified-call-site`.

```sh
PYTHONPATH="$PWD/linters" UV_CACHE_DIR="$PWD/.uv-cache" \
    uv run linters-extra/usecase-module-alias/check.py --self-test
PYTHONPATH="$PWD/linters" UV_CACHE_DIR="$PWD/.uv-cache" \
    uv run linters-extra/usecase-module-alias/check.py
```
