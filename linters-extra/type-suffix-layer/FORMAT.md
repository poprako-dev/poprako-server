# type-suffix-layer

> Forbid mixing model, data, and entity type suffixes across layers.
> Code: `TSL001` | `--fix`: unsupported (check only)

## Why

Role suffixes identify the owning layer of a type. A suffix that leaks into
another layer hides the type's real role and makes layering ambiguous.

## Owned suffixes

| Layer                                            | Suffixes              |
| ------------------------------------------------ | --------------------- |
| `src/model/`                                     | `Info`, `Entry`, `Spec` |
| `src/data/`                                      | `Instr`, `Val`, `View`  |
| `src/part_impl/repo/rdb_impl/entity/`            | `Record`, `Item`, `Pack` |

A declaration inside one of these layers must not use a suffix owned by another
layer.

## Diagnostics

```text
src/model/read/team.rs:3:1: TSL001: type TeamRecord uses suffix Record owned by src/part_impl/repo/rdb_impl/entity/
```

## Exclusions

- Only the `poprako-server` package is inspected; infrastructure crates keep
  their own type contracts.
- Test and mock sources are ignored through the shared production-source mask.
- Declarations outside the three layers are not inspected.
- Only `struct`, `enum`, `union`, and `type` declarations are inspected.
