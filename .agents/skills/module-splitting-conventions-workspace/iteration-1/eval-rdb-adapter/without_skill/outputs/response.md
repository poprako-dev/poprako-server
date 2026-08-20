Refactor the adapter by responsibility, not by preserving a generic `statement`/`impl` split. A suitable layout is:

```text
user.rs                  # module declarations and shared adapter surface
user/
├── account.rs           # account lifecycle, credentials, and profile SQL
├── avatar.rs            # avatar reservation and upload-state SQL
└── coordination.rs      # Run/Step implementations and transaction dispatch
```

`user.rs` should remain a thin parent: declare the three private child modules, re-export only anything genuinely needed outside the adapter, and hold shared imports/types only if more than one child needs them. Move each SQL helper intact into the responsibility that owns it. Move all `Run`/`Step` implementations, transaction-level bounds, operation matching, and calls through `RdbContext` into `coordination.rs`; it may import the narrowly scoped helpers from `account` and `avatar` via `pub(super)` visibility.

Do not split into arbitrary numbered chunks or keep parallel `statement` and `impl` files merely to reduce line count. The boundaries should explain the code: account/profile persistence, avatar persistence, and orchestration. Keep tests beside the responsibility they exercise, or retain a parent `tests` module only for cross-responsibility adapter behavior.

Before implementing, estimate the resulting sizes and split `account.rs` further into `account.rs` plus `profile.rs` if account/profile SQL would itself approach 600 lines. Every resulting Rust source file must be below 600 lines, including tests. The refactor must preserve operation types, trait implementations, transaction isolation bounds, SQL behavior, error mapping, and the public module surface.

After moving code, verify with:

```text
cargo fmt --all --check
cargo check --all-features
cargo test -p poprako-server user
wc -l src/part_impl/repo/rdb_impl/user.rs src/part_impl/repo/rdb_impl/user/*.rs
```

This is a structural refactor only; no production behavior or schema should change.
