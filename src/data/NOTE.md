# NOTE

## Data layer naming

Request-equivalent structures use the `Instr` suffix, direct response-equivalent
structures use `Val`, and response-only nested structures use `View`.

The role is also represented by the submodule path:

- `crate::data::instr::<domain>` for request instructions;
- `crate::data::val::<domain>` for direct response values;
- `crate::data::view::<domain>` for nested response views.
