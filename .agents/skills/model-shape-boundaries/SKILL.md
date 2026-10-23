---
name: model-shape-boundaries
description: Design, migrate, and review owned Model types and borrowed Shape types, normalization accessors, and their repository contracts in the PopRaKo server crate.
---

# Model and Shape boundaries

Read the affected Model, its callers, and both persistence adapters before
changing ownership. These rules apply to the server domain, not infrastructure
crates or adapter-local Diesel rows.

## Choose the owner

`src/model` holds owned business data. Its `read`, `write`, and `shared` roles
classify queried state/specifications, intended changes/imports, and common
concepts. Stored fields must own their data transitively: no references,
`Cow`, lifetime parameters, or aliases hiding borrowed storage. Methods may
borrow from `self`; owning a Model does not require copying it at every layer.

`src/shape` is a sibling of `model`. Use a Shape only for a concrete performance
reason: borrowing existing IDs through a transaction, projecting large edit
batches, or planning imports without copying source/target strings. Document
which owner outlives the Shape and which allocation the borrow avoids.
A string that merely needs `trim()` is not sufficient reason to create a Shape.

Place Shapes under `shape::{read,write,shared}::<domain>` as needed; do not
create empty roles. Name every Shape with a domain-qualified `Shape` suffix
and explicit, genuinely stored lifetimes. Mixed ownership is appropriate:
generated IDs, merged comments, and vectors of references may be owned while
existing strings and models are borrowed. Do not add marker lifetimes merely
to satisfy naming, and do not reintroduce `Cow` for the four creation entries.

Models and values cannot depend on Shapes. Shapes may depend on models and
values, but cannot depend on `complex`, `data`, `part`, `usecase`, `api`,
`extra`, `part_impl`, `harn`, or top-level `shared`. Keep validation and planning
in `complex`, assembly and transaction ownership in `usecase`, and persistence
representations in adapters. Follow
[usecase-boundaries](../usecase-boundaries/SKILL.md) for those contracts.

## Normalize without copying

Move DTO-owned strings and vectors into Models by value. Expose `trimmed_*`
methods returning `&str`, `Option<&str>`, or iterators of borrowed slices when
persistence needs normalized values. Keep the original buffers owned by the
Model. Both RDB and Mock adapters must consume the same normalized view; Mock
then copies only what its independent stored state must own.

Preserve each field's established semantics. `Some("")` and `None` are not
interchangeable: optional term comments and termbase descriptions discard
empty trimmed values, while ordinary profile descriptions preserve presence.
Unit text, notes, and multiline review content preserve original formatting;
trim-based predicates do not authorize trimming persisted text.

Single-term commands use owned `TermEntry`/`TermRepl`. Import planning borrows
owned `TermImport`/`TermbaseImport` and persisted terms into batch Shapes.
Keep their owners alive until the repository operation completes. Move the
page-identifier vector into its Shape while borrowing its strings.

## Verify the boundary

Trace the input buffer through conversion, validation, planning, and adapters.
Review copies for ownership necessity; do not claim allocation savings from
renaming alone. Test meaningful normalized output, empty/absent behavior,
formatting preservation, and pointer identity where borrowing is the contract.
Fixtures that retain related records should store owned projections instead
of self-referential Shapes or artificial `'static` values.

Mechanical checks are documented in repository-relative
`linters-extra/model-shape-boundaries/FORMAT.md` and
`linters-extra/layer-dependency/FORMAT.md`. Follow the root validation rules;
linters supplement review and do not prove allocation counts or business
normalization semantics. Update `docs/ownership-copy-audit.md` when the owner
or justification for an intentional copy changes.
