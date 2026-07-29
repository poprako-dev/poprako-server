# NOTE

## Data layer naming

Request-equivalent structures use the `Instr` suffix, direct response DTOs use
`Val`, and response-only structures use `View`. A direct projection of a model
`*Info` is always an `*InfoView`, including when returned alone or wrapped in a
`Vec`, `Option`, or another response DTO.

The role is also represented by the submodule path:

- `crate::data::instr::<domain>` for request instructions;
- `crate::data::val::<domain>` for direct response values;
- `crate::data::view::<domain>` for nested response views.
