# Inline Proxy Macro Rule

`run_proxy!` and `step_proxy!` create short-lived capability adapters for one
call. They must be constructed directly in that call's argument list.

```rust
PermissionComplex::ensure(
    &mut run_proxy! {
        repo => for<'a> GetTeamInfo<'a>;
    },
    user_id,
)
.await?;
```

Binding either macro to a local variable is forbidden, even when the variable
is consumed only once:

```rust
let mut permission_proxy = run_proxy! {
    repo => for<'a> GetTeamInfo<'a>;
};

PermissionComplex::ensure(&mut permission_proxy, user_id).await?;
```

The checker also rejects returning, assigning, storing, or otherwise wrapping
a proxy macro outside a call argument. Reference expressions (normally `&mut`)
and parentheses are the only wrappers permitted between the macro invocation
and the argument list.

## Checker

```bash
uv run fmt/proxy-macro-inline/check.py --self-test
uv run fmt/proxy-macro-inline/check.py
```

`PRX001` reports a `run_proxy!` or `step_proxy!` invocation that is not inline
in a call argument.
