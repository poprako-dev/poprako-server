<!-- Parent: ../AGENTS.md -->

# src

The root `AGENTS.md` governs this tree. `src/lib.rs` is the authoritative
module graph: HTTP boundaries live in `api/http`, orchestration in `usecase`,
pure rules in `complex`, persisted models in `model`, DTOs in `data`, ports in
`part`, production adapters in `part_impl`, and shared enums/value objects in
`value`.

Keep transaction ownership in use cases through `Nucl::coord`. Do not add
database migration execution to application startup. For changed behavior,
follow the relevant project-local skills and update focused tests before the
broader CI entry points.
