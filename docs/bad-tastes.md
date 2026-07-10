# Bad tastes

## Code style

### Redundant match

Bad:

```rust
match v {
    Ok() => {}
    Err(e) => {
        // ...
    }
}
```

Good:

```rust
if let Err(e) = v {
    // ...
}
```

How to detect:

## Struct usage

### Typed `Result`

No `result` in variable names.
No explict `Result` type hint.
No local variable with `Result` types.

Bad:

```rust

```

How to detect:

## Scope keyword

### Non-plain `pub`

`pub(super)` and `pub(crate)` is unnecessary, whose existence implies the abnormal architect design.

Bad:

```rust
pub(crate) fn some_func() {
    // ..
}

struct SomeStruct {
    pub(super) some_field: String,
}
```

Good:

```rust
fn some_func() {
    // ...
}

struct SomeStruct {
    some_field: String,
}
```

How to detect:
