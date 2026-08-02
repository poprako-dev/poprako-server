---
name: general-conventions
description: Current PopRaKo Rust architecture, naming, construction, and repository-wide source conventions. Use for every creation, refactor, move, or review of Rust code under src/.
---

# Active Rust conventions

## Sources of truth

- Treat `src/lib.rs` as the authoritative module graph and read neighboring
  code before selecting names, visibility, imports, or test placement.
- Apply this skill only to the active `api`, `complex`, `config`, `data`,
  `extra`, `harn`, `model`, `part`, `part_impl`, `result`, `shared`, `usecase`,
  `util`, and `value` architecture.
- Treat `fmt/*/FORMAT.md` and `sh fmt/run-check.sh` as authoritative for
  mechanical layout, imports, identifiers, macro placement, and module
  dependencies. Do not reproduce those checkers manually in a skill.

## Layer and type names

- Persisted projections live under `model::read`; mutation inputs and
  reservations live under `model::write`.
- Request DTOs live under `data::instr` and end in `Instr`.
- Direct response values live under `data::val` and end in `Val`.
- Model `*Info` projections exposed over the API live under `data::view` and
  end in `InfoView`; other nested response structures end in `View`.
- Repository operation descriptors live under `part::repo::oper` and carry
  domain-qualified names. Domain repository capability traits live directly
  under `part::repo`.
- Use specific local names such as `comic_info`, `chapter_entry`,
  `cover_reservation`, and `system_mail_infos`.

## Construction and flow

- Bind domain payloads such as entries, replacements, specs, and DTOs before
  using them. Construct one-shot Orchestra operation descriptors directly in
  the consuming `run_on`, `step_on`, or proxy call as required by `fmt/`.
- Use guard clauses, `match`, and `let ... else`; do not introduce
  `if ... else`.
- Bind the value returned by a transaction before converting or returning it.
  For unit output, await the transaction and then return `accept(())` or the
  nearby equivalent.
- Keep comments and public documentation in English and about current
  behavior. Remove commentary about retired designs instead of preserving it
  in active modules.
- Keep Rust files below the repository limit and extract focused sibling
  modules using `foo.rs` plus `foo/`; do not create `mod.rs`.

## Public contracts and macros

- Keep implementation details private and make a `pub` item a deliberate
  contract.
- Document every public contract and follow the checked source-comment rules.
- Import derive and attribute macros explicitly and call them by their bare
  names. Invoke tracing event macros through `tracing::...!` with structured
  fields.
- Import traits used only for method resolution as `as _`.

## Review

- [ ] Active paths and role suffixes match the current module graph.
- [ ] Domain payloads are named and bound; one-shot opers remain inline.
- [ ] The change does not introduce retired architecture terminology.
- [ ] The project formatter/checker suite is the mechanical source of truth.
