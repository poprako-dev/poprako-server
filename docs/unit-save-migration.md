# Unit Save Logic Migration: PopRaKo-S → PopRaKo-R

This document describes how the **unit save** business logic was migrated from
the original Go implementation (`poprako-s`) to the Rust rewrite
(`poprako-r`). It covers architectural differences, the ordering strategy
change, the transactional flow, and key design decisions.

---

## Table of Contents

- [Architecture Comparison](#architecture-comparison)
- [Ordering Strategy: `CandOrder` vs `before_id`](#ordering-strategy-candorder-vs-before_id)
- [Transactional Flow Comparison](#transactional-flow-comparison)
- [Detailed Layer-by-Layer Migration](#detailed-layer-by-layer-migration)
- [Key Design Decisions](#key-design-decisions)
- [API Contract Differences](#api-contract-differences)
- [Verification](#verification)

---

## Architecture Comparison

| Aspect | Go (poprako-s) | Rust (poprako-r) |
|---|---|---|
| **Architecture** | Domain/App/Infra vertical slices | Ports-and-transaction-steps core |
| **Transaction model** | Manual `RunWithTxn` closure | `Drive::with_context` + generic `Advance<Step, C>` |
| **Repository interface** | Concrete GORM methods | `Step` descriptors + `Execute`/`Advance` traits |
| **Permission checks** | Domain service (`UnitSvc`) | `UnitPermComplex` with proxy execution |
| **Ordering mechanism** | Client-suggested `CandOrder` list | Per-oper `before_id` anchor |
| **Oper discrimination** | Inferred from `local_id` vs `id` presence | Explicit serde `"oper"` tag |
| **Operator comments** | `TranslatorComment`, `ProofreaderComment` | Not implemented |
| **Pre-transaction work** | Arg validation only | Full `prepare_diff` (validation + ID mapping) |

---

## Ordering Strategy: `CandOrder` vs `before_id`

This is the single largest design difference between the two codebases.

### Go (poprako-s): Client-Suggested Complete Ordering

The Go client submits a `cand_order` list — an ordered array of all surviving
unit identifiers (local IDs for new units, server IDs for existing ones):

```go
type UnitDiffVal struct {
    PageId    string       `json:"page_id"`
    Ops       []UnitOpVal `json:"ops"`
    CandOrder []string    `json:"cand_order"`  // full client-suggested order
}
```

The server (`svc/unit.go:reorderUnits`):

1. Sorts all persisted indices by their current `Index`.
2. Replaces `local_id` values in `CandOrder` with real server IDs.
3. Splits the persisted order into two sets: units in `CandOrder` and units
   outside it.
4. Units outside `CandOrder` are distributed into "slots" between cand order
   positions using proportional rank mapping (`rank * nCand / nAll`),
   preserving their relative cluster locality.

**Problem with this approach**: The client must know about *all* units on the
page and submit a complete ordering. If another client concurrently adds a new
unit, the `CandOrder` may conflict. The proportional rank mapping is also
complex and fragile.

### Rust (poprako-r): Per-Oper Relative Anchor

The Rust client does NOT submit a candidate order. Instead, each create/save
oper optionally carries a `before_id` anchor:

```rust
pub enum UnitPreparedOper {
    Create { id, payload, before_id: Option<String> },
    Save   { id, payload, before_id: Option<String> },
    Delete { id },
}
```

The server applies relative positioning per oper:

- `before_id: Some("unit-b")` — place this unit *before* `unit-b`.
- `before_id: None` — place this unit at the tail.
- If `before_id` equals the unit's own ID, or the target ID no longer exists
  (deleted by a prior oper), the unit goes to the tail.

The complete reindex happens inside `UnitComplex::apply_opers_to_order`:

1. Build the current order from persisted indexes (sorted by `index` then `id`).
2. For each oper in submission order:
   - **Create/Save**: Remove the unit from current order if present, then
     insert before `before_id` (or at tail).
   - **Delete**: Remove the unit from current order.
3. Diff the new order against persisted indexes, emit only changed positions.

**Advantages**:

- The client only specifies *relative* position, not absolute ordering.
- Concurrent edits compose better — each oper references a specific anchor.
- The ordering logic is a simple list insertion, not proportional mapping.

---

## Transactional Flow Comparison

### Go: Traditional Dependency Injection

```go
func (a *unitAppImpl) SaveByPage(cx context.Context, currUid string, args *val.SavePageUnitsArgs) app_res.AppRes[val.SavePageUnitsRes] {
    // Pre-transaction: validation
    diff, vfyRe := vfySavePageUnitsArgs(args)
    if vfyRe.IsReject() { return ... }

    // Transactional block
    re, err := repo_iface.RunWithTxn[app_res.AppRes[val.SavePageUnitsRes]](a.txnCtrl, func(prov repo_iface.Prov) (app_res.AppRes[val.SavePageUnitsRes], error) {
        comicRepo := prov.ComicRepo()
        // ... get page (exclusive lock) -> permission -> get chapter -> apply ops -> count -> counters -> touch
    })
}
```

### Rust: Type-Safe Generic Usecase

```rust
pub async fn save_infos<D, C, R>(
    drive: &D,
    repo: &R,
    token: UserToken,
    data: SavePageUnitsData,
) -> RegularResult<SavePageUnitsVal> {
    // Pre-transaction: full prepare_diff (validates IDs, payloads, editor IDs; resolves local IDs)
    let UnitApplyParts { opers, local_id_maps } = ...;

    // Transactional block with type-safe context
    let save_units = drive
        .with_context(async move |context| -> RegularResult<SavePageUnitsVal> {
            let repo = repo.derive_transactional().await;

            // All steps are type-checked Step descriptors
            let page_info = repo.advance(context, &PageStep::get_info_excluded(&page_id)).await?;
            UnitPermComplex::can_user_save_infos(...).await?;
            // ... apply opers via Step descriptors, reindex, count, counters, touch
        })
        .await?;
}
```

| Phase | Go | Rust |
|---|---|---|
| **Arg validation** | `vfySavePageUnitsArgs` — validates page_id, diff presence | Matched in usecase + `diff.into_model()` |
| **Oper validation** | In `ApplyOps` inside transaction | `UnitComplex::prepare_diff` before transaction |
| **Local ID resolution** | Inside transaction, in `ApplyOps` | `UnitComplex::prepare_diff` before transaction |
| **Permission check** | `unitSvc.CanEditPageUnits` inside transaction | `UnitPermComplex::can_user_save_infos` inside transaction |
| **Apply mutations** | Loop calling `unitRepo.Create/Save/Delete` | Loop calling `repo.advance(context, &UnitStep::...)` |
| **Reindex** | `reorderUnits(CandOrder, localToReal, indices)` | `apply_opers_to_order` + `build_index_updates_from_order` |
| **Counters** | Count + set page counters + adjust chapter counters + touch comic | Same sequence via step descriptors |
| **Error mapping** | `app_res.Reject(...)` with HTTP status codes | `RegularError` with `ExpectedVariant` variants |

---

## Detailed Layer-by-Layer Migration

### 1. Model Layer (`model/unit.rs`)

**Go** defines `Unit` as a full aggregate with private fields + getters/setters
in `domain/model/aggr/unit.go`. The transport-layer unit diff uses a unified
`UnitOpVal` struct where the op type is inferred from field presence.

**Rust** uses plain struct types with public fields:

- `UnitInfo` — read model, mirrors DB row, includes `is_translated()` method.
- `UnitPayload` — mutable fields only (no id, page_id, index, timestamps).
- `UnitOper` — **tagged enum** (explicit `Create`/`Save`/`Delete` variants).
- `UnitPreparedOper` — same as `UnitOper` but `Create` carries a server-resolved
  `id` instead of a client `local_id`.

The Rust model is more explicit about type states — a `UnitOper` is the
transport form, `UnitPreparedOper` is the validated-and-resolved form. Go
blurs this by using the same `UnitCre`/`UnitSave`/`UnitDel` structs throughout.

### 2. Data Layer (`data/unit.rs`)

**Go** has `val.UnitOpVal` — a flat struct with `*bool`/`*string`/`*float64`
pointer fields. The op discriminant is inferred:

```go
if op.LocalId != "" { /* CREATE */ }
else if hasUnitSavePayload(op) { /* SAVE */ }
else { /* DELETE */ }
```

**Rust** uses `#[serde(tag = "oper", rename_all = "snake_case", deny_unknown_fields)]`
on the `UnitOperData` enum, making the discriminant explicit on the wire:

```json
{"oper": "create", "local_id": "...", "is_bubble": true, ...}
{"oper": "save",   "id": "...", "is_bubble": true, ...}
{"oper": "delete", "id": "..."}
```

The `into_model()` method converts the transport DTOs into domain types.

### 3. Complex (Domain Logic) Layer (`complex/unit.rs`)

**Go** splits domain logic between:

- `unitSvc.ApplyOps` — applies ops + validates `CandOrder` + reindexes.
- `unitSvc.CanListPageUnits` / `CanEditPageUnits` — permission checks.
- `unit_util.go:asmUnitDiff/asmUnitOp` — converts transport diff to domain diff.

**Rust** consolidates this into:

- `UnitComplex::prepare_diff` — validates the entire diff before the
  transaction starts. Generates snowflake IDs for `Create` opers and builds
  `UnitIdMapper` entries. Returns `UnitApplyAck`.
- `UnitComplex::apply_opers_to_order` — applies `before_id` ordering logic.
- `UnitComplex::build_index_updates_from_order` — computes minimal index
  updates.
- `UnitPermComplex::can_user_list_infos` / `can_user_save_infos` — proxy-based
  permission checks.

**Key change**: Validation moves from *inside* the transaction (Go) to
*before* the transaction (Rust). This means IO fails faster (without
consuming a database connection) and the transaction stays shorter.

### 4. Usecase Layer (`usecase/unit.rs`)

**Go** has `unitAppImpl.SaveByPage` — a method on a struct with injected
dependencies. Transaction management uses a `RunWithTxn` closure pattern.

**Rust** has a free generic function `save_infos<D, C, R>` — no struct, the
generic bounds on `R` express exactly which repos the usecase needs.

The Rust usecase is also slightly more explicit about the page lock: it uses
`PageStep::get_info_excluded` (exclusive row lock) vs Go's
`pageRepo.GetByIdEx`.

### 5. Repository Step Layer (`part/repo/step/unit.rs`)

**Go** has a concrete interface `UnitRepo` with methods like
`Create(*aggr.UnitCre)` and GORM is passed directly.

**Rust** defines individual `Step` types — each is a struct implementing the
`Step` trait with an associated `Output` type. This lets the infrastructure
layer (`part_impl`) implement each step independently.

There are 8 step types for the unit domain:

| Step | Output | Used in |
|---|---|---|
| `ListInfosByPageId` | `Vec<UnitInfo>` | `list_infos` usecase |
| `ListAllInfosByPageId` | `Vec<UnitInfo>` | Mock/internal queries |
| `CreateInfo` | `()` | `save_infos`, `Create` oper |
| `SaveInfo` | `()` | `save_infos`, `Save` oper |
| `DeleteByIdInPage` | `()` | `save_infos`, `Delete` oper |
| `ListIndexesByPageId` | `Vec<UnitIndex>` | `save_infos`, reindexing |
| `UpdateIndexesByPageId` | `()` | `save_infos`, reindexing |
| `CountByPageId` | `UnitCounters` | `save_infos`, counter sync |

A `UnitStep` factory struct provides constructor methods for all step types.

### 6. Repository Trait Layer (`part/repo/unit.rs`)

Two traits with generic `C` (context type parameter):

- `UnitRepo<C>` — non-transactional: only `ListInfosByPageId` +
  `ListAllInfosByPageId` execute bounds.
- `UnitRepoTransactional<C>` — transactional: all 8 step `Advance` bounds.

This split mirrors Go's interface but with type-system safety instead of
runtime checks.

### 7. Infrastructure Layer (`part_impl/repo/rdb_impl/unit.rs`)

| Operation | Go (GORM) | Rust (Diesel) |
|---|---|---|
| **Create** | `db.Create(&entity.UnitRow{...})` | Get `next_index` (max+1 or 0), build `UnitEntry`, insert |
| **Save** | `OnConflict{UpdateAll: true}` | Check existence → if missing: create; if same page: update; if different page: error |
| **Delete** | `db.Where("page_id").Delete(...)` | `diesel::delete(filtered_by_page_id_and_id)` |
| **Reindex** | `CASE WHEN id=? THEN ? ...` SQL | Two-phase: shift all +100,000, then set target indexes |
| **Count** | SQL `COUNT(CASE WHEN ...)` | Read all rows into memory, fold in Rust |

### 8. Mock Layer (`part_impl/repo/mock_impl/unit.rs`)

An in-memory mock using `Mutex<Vec<UnitInfo>>` that mirrors the RDB
implementation. This enables unit testing the usecase without a database.

### 9. API Handler Layer (`api/http/handler/unit.rs`)

| Aspect | Go | Rust |
|---|---|---|
| **Route** | `POST /api/v1/units?page_id=X` | `POST /api/v1/pages/{page_id}/units/save` |
| **page_id location** | Query parameter + body override | Path parameter validated against body |
| **Auth** | Cookie + header fallback | Axum middleware `Extension<UserToken>` |

The Rust handler validates that `path page_id == body data.page_id == body
data.diff.page_id` (three-way match) using `ensure_path_matches_body_id`,
returning 422 on mismatch.

---

## Key Design Decisions

### 1. Before-transaction validation

**Decision**: `UnitComplex::prepare_diff` runs entirely before the database
transaction.

**Why**: Go performs `CandOrder` validation and local ID resolution *inside*
the transaction, keeping the transaction open during CPU-bound work. Moving
this step out shortens the write transaction window, reducing lock contention.

### 2. `before_id` instead of `CandOrder`

**Decision**: Per-create/save relative positioning instead of a complete
client-suggested order list.

**Why**:

- Eliminates the complex `CandOrder` → `reorderUnits` proportional mapping.
- Each oper is self-contained — no need to coordinate across all ops.
- Concurrent edits compose more naturally (each oper only anchors on one ID).
- Reduces client payload size (no need to send the full unit list).

### 3. Explicit serde tag instead of inference

**Decision**: `#[serde(tag = "oper")]` on the `UnitOperData` enum.

**Why**: Go's field-presence inference is fragile — adding a new mutable field
can change how a payload is classified. The explicit tag makes the op
discriminant unambiguous on the wire, at the cost of one extra JSON field.

### 4. Complete payload in every write oper

**Decision**: Every `Create` and `Save` oper carries a **complete** unit
payload (all mutable fields: `is_bubble`, `is_proofread`, `x_coord`,
`y_coord`, `translated_text`, `last_translator_id`, `proofread_text`,
`last_proofreader_id`).

**Why**: Concurrent replay. If another client deletes a unit between a
client's diff submission and server processing, a `Save` or
`move_before`-equivalent oper can still restore the unit from the complete
payload. Partial-payload updates would lose data when the anchor unit is
missing.

### 5. Omitted `TranslatorComment` and `ProofreaderComment`

**Decision**: These Go fields are not implemented in the Rust model.

**Why**: The translator/proofreader comment fields add complexity to the
model, DTOs, and storage with no current frontend consumer. They can be added
as nullable columns in a future migration when a consumer exists.

### 6. Two-phase reindex instead of CASE-WHEN

**Decision**: The Diesel reindex shifts all affected rows by +100,000 in one
UPDATE, then sets each row to its target index in a second UPDATE.

**Why**: PostgreSQL enforces unique constraints on `(page_id, index)`. A
naive `CASE WHEN` loop risks transient constraint violations. The two-phase
approach avoids this without requiring `DEFERRABLE` constraints.

---

## API Contract Differences

### Request Bodies

**Go** (`POST /api/v1/units?page_id=X`):
```json
{
  "diff": {
    "page_id": "page-1",
    "ops": [
      {
        "local_id": "temp-1",
        "is_bubble": true,
        "is_proofread": false,
        "x_coord": 12.5,
        "y_coord": 33.0,
        "translated_text": "hello",
        "last_translator_id": "user-1",
        "proofread_text": null,
        "last_proofreader_id": null
      }
    ],
    "cand_order": ["temp-1"]
  }
}
```

**Rust** (`POST /api/v1/pages/{page_id}/units/save`):
```json
{
  "page_id": "page-1",
  "diff": {
    "page_id": "page-1",
    "opers": [
      {
        "oper": "create",
        "local_id": "temp-1",
        "is_bubble": true,
        "is_proofread": false,
        "x_coord": 12.5,
        "y_coord": 33.0,
        "translated_text": "hello",
        "last_translator_id": "user-1",
        "proofread_text": null,
        "last_proofreader_id": null
      }
    ]
  }
}
```

Key differences:

1. **page_id in the URL path**, not a query parameter.
2. **Explicit `"oper"` tag** on each operation (`"create"`, `"save"`,
   `"delete"`).
3. **No `cand_order`** — ordering is expressed inline via optional
   `before_id` on each create/save oper.
4. **`local_id` and `id` are explicitly scoped** — `local_id` for create
   opers, `id` for save/delete opers. Not inferred from field presence.
5. **`deny_unknown_fields`** is set on the serde deserializer, so unknown
   fields in the request body are rejected.

### Response Bodies

**Go**:
```json
{
  "total_unit_count": 3,
  "translated_unit_count": 2,
  "proofread_unit_count": 1
}
```

**Rust**:
```json
{
  "local_id_mappers": [
    { "local_id": "temp-1", "unit_id": "snowflake-id-abc" }
  ],
  "total_unit_count": 3,
  "translated_unit_count": 2,
  "proofread_unit_count": 1
}
```

The Rust response adds `local_id_mappers` — a mapping from each `local_id`
submitted in `Create` opers to the server-generated ID. This lets the client
associate its optimistic UI elements with the persisted server identifiers.

---

## Verification

### Unit tests for the Rust unit save flow

| Location | Focus |
|---|---|
| `src/complex/unit/tests.rs` | `prepare_diff` validation, order operations, index building |
| `src/usecase/unit/tests.rs` | Full save usecase with mock repos, concurrent merge scenarios |
| `src/data/unit/tests.rs` | Serde roundtrip, `into_model` conversions |
| `src/part_impl/repo/rdb_impl/unit/tests.rs` | Integration test with test database |

### Manual verification

```bash
# Generate the OpenAPI spec (requires swagger-ui feature)
cargo run --features swagger-ui -- --swagger > docs/swagger.json

# Verify compilation
cargo check

# Run unit-related tests
cargo test -- unit
```

---

## Reference

- Go business reference: `references/poprako-s/internal/app/impl/unit.go`
- Go domain service: `references/poprako-s/internal/domain/svc/unit.go`
- Rust usecase: `src/usecase/unit.rs`
- Rust complex: `src/complex/unit.rs`
- Rust model: `src/model/unit.rs`
- Rust data: `src/data/unit.rs`
- Rust API handler: `src/api/http/handler/unit.rs`
- Rust OpenAPI registration: `src/api/http/openapi.rs`
- API spec document: `docs/unit-save-api.md`
