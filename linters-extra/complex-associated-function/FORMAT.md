# Complex free-function interface

Stateless domain behavior in the public module interface rooted at
`src/complex.rs` uses free functions. Do not wrap these functions in empty
`*Complex` namespace structs or inherent impls. Keep structs that carry data or
express type semantics, including trait implementations and instance methods.

Import public behavior modules under descriptive snake_case `*_complex` aliases
and qualify their functions through those aliases. Preserve the established
behavior names: `team_complex`, `team_perm_complex`, `chapter_artwork_complex`,
`chapter_translation_import_complex`, and `chapter_translation_export_complex`.
The alias describes the behavior rather than mechanically copying its file path.

```rust
// Required: free functions in src/complex/team/perm.rs.
pub fn check() {}

// Required: a module alias at the caller.
use crate::complex::team::perm as team_perm_complex;

fn authorize() {
    team_perm_complex::check();
}
```

Do not import public complex free functions directly, rename them individually,
or glob-import a behavior module. Do not call through an unaliased module or
through a longer qualified path such as `team_complex::perm::check()`; import
the permission module as `team_perm_complex` instead. Function references use
the same convention as calls.

Data types may still be imported directly. Private helper modules may retain
ordinary function imports, and local function calls inside the defining module
need no alias. Keep the existing domain responsibilities and public visibility
boundaries when replacing namespace structs.

## Checker

The checker scans production Rust under `src/`, using the shared production
source masker to exclude tests and mocks. It discovers the public complex
module graph, including inline modules, and its public free functions.

- `CPX001`: empty `*Complex` namespace struct without a trait implementation or
  instance method in the same file and module.
- `CPX002`: direct or glob import of public complex behavior functions.
- `CPX003`: import of a behavior module without a snake_case `*_complex` alias.
- `CPX004`: qualified reference to a public complex free function that does not
  use a direct `*_complex::function` path.

This syntax checker validates alias shape, not domain-specific alias spelling.
It resolves ordinary nested use trees and lexical import scopes; it does not
expand macros, infer types, or resolve arbitrary re-export chains. Re-export
restrictions are checked separately by `qualified-call-site`.

The existing checker directory remains the CI entry point:

```sh
PYTHONPATH="$PWD/linters" UV_CACHE_DIR="$PWD/.uv-cache" \
    uv run linters-extra/complex-associated-function/check.py --self-test
PYTHONPATH="$PWD/linters" UV_CACHE_DIR="$PWD/.uv-cache" \
    uv run linters-extra/complex-associated-function/check.py
```
