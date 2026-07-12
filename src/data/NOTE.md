# NOTE

## Data structs must not contain domain names.

For example, `InfoVal` not `TeamInfoVal`.

The way to prevent ident naming conflicts is use `team_data::` prefix anywhere such `InfoVal` is referenced.

## Input structs should end with `Data`, while output structs ends with `Val`

For example, `InfoUpdateData` and `InfoVal`.

This is reflection of functional styles.
