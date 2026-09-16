# Server module map

Use `src/lib.rs` for the authoritative module graph. HTTP delivery lives in
`api/http`, domain orchestration in `usecase`, pure rules in `complex`,
persisted models in `model`, DTOs in `data`, ports in `part`, adapters in
`part_impl`, background scheduling in `extra`, and shared domain types in
`value`. `harn.rs` stores the already-composed application parts.

Read the root skill routing for the affected layer. Workspace utility and
infrastructure crates have their own module graphs in their crate roots.

## Domain rule interfaces

Expose stateless domain operations in `complex` as module-level free functions.
Do not introduce empty `*Complex` structs solely to hold associated functions.
Keep types that carry data or provide meaningful type-system contracts.

Import domain-operation modules with explicit `*_complex` aliases and call
functions through those aliases, for example `user_complex::hash_password`
and `team_perm_complex::ensure_user_can_create`. Permission operations belong
in the entity's `perm` module, imported as `team_perm_complex` for teams.
Keep domain data types imported by their own names and keep local helper calls
local. This interface convention does not change layer or transaction ownership.

## Use-case call sites

Import use-case modules with descriptive `*_usecase` aliases and invoke their
functions through those aliases, such as `user_usecase::get_info`. Import
nested operation modules directly, for example `user::delete` as
`user_delete_usecase` and `chapter::stage` as `chapter_stage_usecase`.
Do not import use-case functions directly or call them through `usecase::...`
paths. Data types, view construction helpers, and private implementation helpers
keep their existing import conventions; local calls and module-local tests may
use local function names. Keep use cases as free functions and preserve their
domain and transaction boundaries.
