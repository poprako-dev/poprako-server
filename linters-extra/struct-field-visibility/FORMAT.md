# Uniform struct field visibility

A struct either exposes every field or exposes none. Mixed field visibility
means the struct is neither a data shape nor a behaviour object:

- Data-oriented structs expose every field so consumers can destructure or
  build them directly.
- Behaviour-oriented structs keep every field private and expose operations
  instead.

Restricted visibility such as `pub(crate)` or `pub(super)` counts as exposed;
the rule classifies a field by whether it carries any visibility modifier, not
by the level of that modifier. Unit structs and structs without fields are
always valid.

Tuple structs are checked as well, because positional destructuring has the
same requirement.

## Diagnostic

`SFV001`: the struct mixes exposed and private fields. Decide which kind of
struct it is and make the visibility uniform:

```rust
// BAD
pub struct ComicInfoView {
    pub id: String,
    title: String,
}

// GOOD (data-oriented, destructurable by consumers)
pub struct ComicInfoView {
    pub id: String,
    pub title: String,
}

// GOOD (behaviour-oriented, fields stay internal)
pub struct ComicInfoView {
    id: String,
    title: String,
}

impl ComicInfoView {
    pub fn id(&self) -> &str {
        &self.id
    }
}
```

A violation usually means one field was made public just to serve one caller
while the rest stayed private. Prefer moving the operation into the module, or
exposing the whole data shape when the type is genuinely a data struct.

Test-only source receives no diagnostics.

## Interaction with minimum-pub

`minimum-pub` judges struct fields as one unit: when any field of a struct is
referenced outside its defining module, every field of that struct may stay
`pub`. Together the two rules mean a data struct's fields are uniformly public,
and one genuinely shared field justifies the exposure of its siblings.

## Running

```sh
linters-extra/.venv/bin/python linters-extra/struct-field-visibility/check.py --self-test
linters-extra/.venv/bin/python linters-extra/struct-field-visibility/check.py --root .
sh linters-extra/run-check.sh
```
