---
name: implement-fullchain-spec
description: Current PopRaKo vertical-slice workflow across model, data, complex, Orchestra operations, adapters, use cases, HTTP, migrations, and tests. Use for every new or changed backend behavior.
---

# Vertical-slice workflow

Add only the layers the behavior needs, but keep every affected contract
consistent from persistence to HTTP.

## 1. Establish behavior

1. Read the current domain code, active business documentation, and nearby
   tests.
2. Record permissions, transaction boundaries, side effects, uniqueness and
   locking rules, response shape, and negative cases.
3. Find a completed operation with the same execution shape.

## 2. Domain and transport types

- Put shared small concepts under `value`.
- Put persisted projections and list specs under `model::read`; put entries,
  modifications, replacements, and reservations under `model::write`.
- Put pure rules and permission helpers under `complex`. They receive loaded
  concrete model evidence and must not import Orchestra, repository traits or
  operations, Prom operations/tasks, or drive transactions.
- Put request DTOs under `data::instr`, direct response values under
  `data::val`, and model projections under `data::view`. Convert timestamps to
  Unix milliseconds at the response boundary.

## 3. Repository operations and adapters

1. Define a domain-qualified descriptor under `part/repo/oper/<domain>.rs`
   with `#[oper(output = ...)]`.
2. Add it to the domain `XxxRepo<C>` capability using `#[drive(...)]`, choosing
   `run` for an independent operation and `step` for an operation inside a
   caller-owned transaction.
3. Implement `Run<Oper>` or `Step<Oper, Context>` for the RDB adapter and the
   mock adapter needed by use-case tests.
4. Keep RDB Orchestra implementations beside focused SQL helpers. Map Diesel
   failures through `crate::shared::result`.
5. Keep Diesel entities under `part_impl/repo/rdb_impl/entity`. Change
   migrations and regenerate `schema.rs`; never edit generated schema by hand.

## 4. Use case and side effects

- Implement public generic orchestration functions under `usecase`.
- Use `.run_on(repo)` for independent operations and `.step_on(repo, context)`
  inside `Nucl::coord`. Construct operation descriptors inline.
- Use private `usecase::internal::*Loader` associated methods only for reusable
  chains containing at least two repository reads. Model-returning method names
  include the model suffix (`load_info_from_*`, `load_infos_from_*`, and so on).
- Pass repositories independently from `LoadMode`; use `LoadMode::Run` only
  when every operation in the chain supports `Run`, and `LoadMode::Step` with
  the caller-owned context when every operation supports `Step`.
- Do not create loaders for one-operation ID lookups. Do not pass IDs already
  carried by an input model, and return missing required models from the loader
  before invoking pure `complex` rules.
- Persist deferred `Prom` work through `Defer` or `DeferBatch` in the owning
  transaction. Respect its at-least-once delivery contract and make handlers
  idempotent.
- Emit immediate effect events only after transaction semantics are clear.
- Keep permissions in use cases or pure complex helpers, never in handlers or
  RDB adapters.

## 5. HTTP exposure

1. Add or update the handler, instrument it with the nearby `skip_all` style,
   and propagate the use case with `?`.
2. Validate transport-only facts in the handler and build success responses
   through `Accept as _` or `no_content`.
3. Keep router paths, handler extractors, `#[utoipa::path]`, and OpenAPI schema
   registration aligned.

## 6. Validation

- Add focused positive and relevant negative Rust tests.
- Add RDB coverage for constraints, locking, queries, or transaction behavior.
- Update the TypeScript HTTP suite and `TESTCASES.md` together when the public
  behavior changes.
- Run formatting checks, targeted tests, workspace checks, and broader tests in
  proportion to the affected surface.
