---
name: data-dto-boundaries
description: Define and review PopRaKo data DTO boundaries under src/data. Use this skill whenever creating, changing, moving, or reviewing Instr, Val, or View types, their fields, conversions, serialization, or composition. It establishes View as the smallest request/response-independent data unit, Instr as request-only, and Val as response-only; always apply it when judging dependencies between these DTO categories.
---

# Data DTO boundaries

Use this skill for every change to `src/data/instr`, `src/data/val`, or
`src/data/view`. The three DTO categories have different contracts. Keep those
contracts explicit when choosing a type, moving a field, or reviewing an
import.

## Core model

`View` is the smallest reusable data unit in the data layer. It describes a
presentation fragment without knowing whether it is used in a request or a
response. A View must not encode request intent, mutation instructions, or
endpoint-specific response meaning.

`Instr` is request-only. It represents input supplied to a use case or HTTP
endpoint and may contain request-specific validation and intent. Do not use an
Instr as a response payload or as a reusable presentation fragment.

`Val` is response-only. It represents the direct result of a use case or
endpoint and may aggregate several Views, collections of Views, optional
Views, counters, identifiers, and other response-specific fields. A Val is the
correct owner for endpoint/use-case response meaning.

The intended dependency direction is:

```text
model/value/domain data ──> View ──> Val
request-specific data ───> Instr
```

The diagram describes semantic ownership, not a requirement that every Val
must contain a View. A Val may consist entirely of scalar fields. A View may
be nested inside another View when the nested value is still an independent,
request/response-neutral presentation fragment.

## Dependency rules

- `View` must not import or contain `Instr` or `Val`.
- `Instr` must not import or contain `View` or `Val`.
- `Val` may import and contain `View`; this is the expected way to reuse
  response-neutral fragments in response payloads.
- `Val` must not import or contain `Instr`.
- A View may convert from a model projection or shared value object. That
  conversion does not make the View request- or response-specific.
- Keep endpoint-specific aggregation in `Val`, not in a reusable View.
- Do not rename a response aggregate to `View` merely because most of its
  fields are Views.

Therefore, `val -> view` is valid and is not a reverse dependency violation.
The prohibited direction is `view -> val` (and the analogous `view -> instr`).

## Naming and placement

- Request DTOs live under `data::instr` and end in `Instr`.
- Direct response DTOs live under `data::val` and end in `Val`.
- Reusable response-neutral fragments live under `data::view` and normally end
  in `View`.
- A model `*Info` projection exposed as a reusable API fragment lives under
  `data::view` and ends in `InfoView`.
- Put a type in `View` when its shape can be reused without carrying the
  meaning of a particular request or response operation.
- Put a type in `Val` when its shape answers a particular use-case or endpoint,
  including list metadata, positional alignment, operation results, and
  upload reservations.
- Put a type in `Instr` when its fields describe what the caller asks the
  system to do, rather than what the system returns.

## Examples

These are valid compositions:

```rust
pub struct ListComicInfosVal {
    pub comics: Vec<ComicInfoView>,
    pub pinned_chapters: Vec<Option<ChapterInfoView>>,
}
```

```rust
pub struct ReserveUserAvatarVal {
    pub slot: Option<ImageUploadSlotView>,
}
```

These are invalid boundaries:

```rust
pub struct UserInfoView {
    pub result: CreateUserVal,
}
```

```rust
pub struct CreateUserInstr {
    pub preview: UserInfoView,
}
```

The first makes a reusable View depend on response-specific meaning. The
second makes request input depend on a presentation/response fragment.

## Review procedure

When reviewing a data DTO change:

1. Identify whether the type describes caller intent, a reusable fragment, or
   a use-case result.
2. Check its module and suffix against that role.
3. Inspect imports and field types for prohibited `View`/`Instr`/`Val`
   directions.
4. Confirm that any aggregation-specific fields remain on the `Val`.
5. Treat `Val` fields containing `View` as valid unless the View itself has
   acquired endpoint-specific meaning.
6. Check nearby DTOs and `src/data.rs` before introducing a new category or
   dependency pattern.

When reporting a dependency finding, distinguish these cases explicitly:

- `view -> val`: invalid reverse dependency.
- `view -> instr`: invalid request/response coupling.
- `instr -> view` or `instr -> val`: invalid request DTO coupling.
- `val -> view`: valid response composition.
- `val -> instr`: invalid response/request coupling.
