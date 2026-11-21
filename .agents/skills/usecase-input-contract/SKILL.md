---
name: usecase-input-contract
description: Enforce the PopRaKo usecase signature contract — deps tuple plus user token plus one owned Instr — and path-versus-payload comparison in the HTTP boundary.
---

# Use-case input contract

This specializes [usecase-boundaries](../usecase-boundaries/SKILL.md) and
[data-dto-boundaries](../data-dto-boundaries/SKILL.md). Follow those skills for
transaction ownership, repository contracts, and DTO roles.

## Core signature

Every use case that has an instruction object uses exactly this shape:

```rust
pub async fn operation<...>(
    (dep_a, dep_b): (&A, &B),
    token: UserToken,
    instr: OperationInstr,
) -> BaseRest<OperationVal>
```

- The dependency tuple is the only borrowed fixed parameter set.
- `UserToken` is the only other fixed non-instruction parameter.
- Every remaining input lives in one owned `OperationInstr`: path identifiers,
  query filters, body payload, credentials, upload entries, and options.
- Bare parameters are allowed only for an extremely simple use case that has no
  instruction object at all. That is an exception, not the default.

Do not write signatures such as:

```rust
pub async fn get_comic(
    (repo, cache): (&R, &K),
    token: UserToken,
    pica_token: PicaToken,
    comic_id: String,
    instr: GetComicInstr,
) -> BaseRest<ComicInfoView>
```

The credential and `comic_id` belong in `GetComicInstr`.

## Owned inputs only

The use case accepts owned data. Do not accept borrowed slices, `&str`, `&T`,
`Option<&T>`, `Cow`, or borrowed shapes as operation inputs:

```rust
// Bad: borrowed operation input.
pub async fn upload_chapter_images(..., images: &[PicaImageEntry]) {}

// Good: the slice is owned by the instruction.
pub async fn upload_chapter_images(
    (relay, bucket): (&L, &Q),
    token: UserToken,
    instr: UploadPicaChapterImagesInstr,
) {}
```

Internal calls from the use case to a port may borrow from the owned
instruction. The restriction applies to what the use case accepts as an
operation input.

## Delivery owns comparison

The HTTP layer extracts transport values, builds the instruction, and compares
path parameters with payload fields. The use case only sees the finished
instruction.

When the operation has a payload, the payload owns the identifier. The path
parameter is a comparison input only:

```rust
let pica_token = extract_pica_token(&headers)?;

if instr.pica_comic_id != pica_comic_id {
    return Err(HttpError::unprocessable(
        "pica_comic_id path and payload mismatch",
    ));
}

let mut instr = instr;
instr.pica_token = pica_token;

relay_pica_usecase::create_chapter(
    (harn.relay(), harn.bucket()),
    user_token,
    instr,
)
.await?;
```

When there is no payload, the delivery layer fills the path identifier and the
credential into the instruction before calling the use case. The use case still
receives only the instruction.

## Instruction shape

- Put instructions under `data::instr` and end every name in `Instr`.
- Put the operation-owned identifier in the payload instruction when a payload
  exists; deserialize it normally and compare it in the handler.
- Pure path or query identifiers that are not payload fields use
  `#[serde(skip)]` and are filled by the delivery layer.
- Credentials such as `PicaToken` use `#[serde(skip)]` and are filled by the
  delivery layer. They never appear as separate use-case parameters.
- Keep all instruction fields owned (`String`, `Vec<T>`, model entries,
  timestamps as owned values). Do not put references in an instruction.
- Use `#[serde(deny_unknown_fields)]` for JSON bodies and
  `#[serde(default, deny_unknown_fields)]` for query instructions.

## Model role types stay in model

Data instructions must not declare or carry model `Entry`, `Info`, or `Spec`
values. `Entry` is owned by `model::write`. When delivery data must cross the
use-case boundary, define a data `*Instr` payload and convert it to the model
inside the use case.

```rust
// Bad: data instruction carries a model Entry.
pub struct UploadPicaChapterImagesInstr {
    pub images: Vec<PicaImageEntry>,
}

// Good: data instruction carries its own owned Instr payload.
pub struct PicaImageUploadInstr {
    pub file_name: String,
    pub content_type: String,
    pub path: PathBuf,
    pub size: u64,
}

pub struct UploadPicaChapterImagesInstr {
    pub images: Vec<PicaImageUploadInstr>,
}
```

The use case builds `PicaImageEntry` from `PicaImageUploadInstr` immediately
before calling the port.

## Public names are PopRaKo names

Do not copy upstream API shorthand into public instructions. For example, an
upstream `q` filter becomes `fuzzy_title`; `status`, `f`, `sort`, and `sh` are
not exposed unless they have a PopRaKo domain meaning and type.

```rust
// Bad: upstream shorthand leaked into the public API.
pub struct ListComicInfosInstr {
    pub q: String,
    pub offset: u32,
    pub status: String,
    pub f: String,
}

// Good: PopRaKo list vocabulary.
pub struct ListPicaComicInfosInstr {
    #[serde(skip)]
    pub pica_token: PicaToken,
    pub fuzzy_title: Option<String>,
    pub offset: u32,
}
```

## Checklist

1. Is the signature `(deps), UserToken, Instr`?
2. Is every other input owned by the instruction, including credentials and
   path identifiers?
3. Does the HTTP handler build or fill the instruction and perform
   path-versus-payload comparison before calling the use case?
4. Is a mismatch rejected in the API layer with an argument error, not inside
   the use case?
5. Are public filter names PopRaKo vocabulary rather than upstream shorthand?
6. Does any data instruction avoid model `Entry`, `Info`, and `Spec` values?
7. Does the use case avoid importing HTTP, filesystem, adapter, or composition
   types?
