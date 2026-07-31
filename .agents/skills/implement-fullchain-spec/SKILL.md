---
name: implement-fullchain-spec
description: Active PopRaKo vertical-slice checklist: model/value/complex, repository steps and adapters, use case, HTTP, and tests. Use for new or changed backend behavior.
---

# Active Vertical-Slice Workflow

Use this workflow for a user-visible behavior in the active ports-and-steps architecture. It is deliberately conditional: do not add layers a behavior does not need.

## 1. Establish the behavior

1. Read current Rust code in the target domain and the corresponding Go business reference.
2. Record perms, transaction boundary, side effects, uniqueness rules, response shape, and negative cases.
3. Find a nearby completed use case with the same operation shape.

## 2. Domain and transport types

- Add a `value` type only for a focused shared concept.
- Add persisted application state and forms/updates under `model`.
- Put pure validation, ordering, and perm predicates under `complex`. It must not execute `Drive`, `Advance`, repository transactions, or prom operations.
- Add request `*Data` and response `*Val` types under `data`. Convert model timestamps to Unix milliseconds in `Val` conversions.

## 3. Repository surface and adapters

1. Add a step descriptor under `part/repo/step/<domain>.rs`.
2. Add its `XxxRepo<C>` and/or `XxxRepoTransactional<C>` bound under `part/repo/<domain>.rs`.
3. Implement matching `Execute<S>` or `Advance<S, C>` for the RDB adapter (`part_impl/repo/rdb_impl`) and mock adapter (`part_impl/repo/mock_impl`) when the behavior is tested without PostgreSQL.
4. RDB entities belong under `part_impl/repo/rdb_impl/entity`; generated Diesel schema remains generated. Never edit `schema.rs` directly.

## 4. Use case and side effects

- Public use cases are free generic functions under `usecase`.
- Use `Execute` for independent operations. Use `Drive::with_context` and `Advance` when several writes or locks must commit atomically.
- Bind the transaction result before returning. Schedule deferred image work through `Prom`; emit effects through the correct effect port only after transaction semantics are established.
- Keep perm checks at the usecase boundary or in a pure perm complex helper, never in the HTTP handler or RDB adapter.

## 5. HTTP exposure

1. Add a handler under `api/http/handler` with `#[instrument(err, skip(...))]`.
2. Use `Accept as _`, propagate the usecase with `?`, and validate only request-boundary facts there.
3. Register the route in `router.rs` and matching `#[utoipa::path]` metadata.
4. Register new OpenAPI schemas in `openapi.rs` when required.

## 6. Validation

- Add focused positive and negative Rust tests beside the target module.
- Add or update integration coverage and `tests/integration-tests/TESTCASES.md` for an HTTP behavior.
- Run formatting, targeted tests, then `cargo check`; run broader tests for shared ports, transactions, or HTTP changes.
