# Import style

Production Rust imports must use explicit leaves and must qualify every path
into the current crate with `crate::`.

## Rules

- `IMPORT001`: wildcard imports are forbidden in production source.
- `IMPORT002`: `self::`, `super::`, the package name, and bare local module
  paths are forbidden in production imports; use `crate::...`.
- `IMPORT003`: production Rust source must parse without tree-sitter `ERROR` or
  `MISSING` nodes so unsupported or malformed syntax cannot silently bypass
  import rules.

Every leaf of a root or nested use list is checked independently. The current
library crate name is read from `Cargo.toml` (`[lib].name`, or the normalized
package name), raw identifiers are normalized, and `extern crate self as ...`
aliases count as current-crate roots. Bare local roots are derived only from
direct module-scope declarations; declarations inside nested inline modules do
not make a same-named external crate look local.

The package library name remains valid in `src/main.rs`: Cargo compiles the
binary and library as separate crates, so `crate::` in the binary cannot name
library items.

Files and modules compiled only with `cfg(test)` are masked before checking, so
test code may use `super`. Mock adapters are production-feature source and are
checked normally. Generated `schema.rs` files receive no special exemption.
