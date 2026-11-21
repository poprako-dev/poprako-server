# No manual serde rename

Manual `#[serde(rename = "...")]` attributes are forbidden. Name the Rust
field or enum variant after its serialized form instead. Container-level
`rename_all` and `rename_all_fields` are casing conversions owned by serde and
remain allowed.

## Diagnostic

`SRN001`: a manual serde rename is present. Remove the `rename` clause and
rename the Rust item, or use an allowed `rename_all` rule.

```rust
// BAD
#[serde(rename = "image_version")]
pub image_ver: u32,

// GOOD
pub image_version: u32,
```

Split forms such as `#[serde(rename(serialize = "...", deserialize = "..."))]`
are also rejected.

## Exceptions

The following cases keep manual renames because their wire values cannot be
expressed by a Rust identifier or `rename_all`:

- Pica upstream wire structs under `src/part_impl/relay/pica/`.
- Enum variants whose serialized value contains a dot or hyphen, such as
  `comic.workset.team` and `label-plus`. If a file contains at least one such
  value, its manual renames are exempt so the enum's ordinary variants can
  stay alongside them.
- Wire names containing a word prohibited by `[forbidden-identifiers]` in
  `rust-style-lint.toml`, such as `image_version` and `queue_capacity`. The
  Rust identifier cannot use the wire name, so the explicit rename remains the
  only representation.

Configuration fields, DTO fields, and ordinary enum variants must use their
wire names directly. Test-only source receives no diagnostics.

## Running

```sh
linters-extra/.venv/bin/python linters-extra/serde-rename-ban/check.py --self-test
linters-extra/.venv/bin/python linters-extra/serde-rename-ban/check.py --root .
sh linters-extra/run-check.sh
```
