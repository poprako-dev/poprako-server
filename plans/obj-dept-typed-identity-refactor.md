# ObjDept Typed Identity Refactor Plan

Status: prepared for implementation. This plan covers object identity,
physical keys, metadata, and durable Check/Delete tasks across ObjDept, its
macros, and the six server object registrations.

The target is one-way physical-key generation and object-kind preservation
through the complete Rust interface. Business identity must never be recovered
from an OSS path. Existing paths, task identities, and lifecycle behavior remain
compatible with persisted data.

## Current gaps

- `poprako-obj-dept/src/key.rs`: `KeyMap` requires `reverse`; `ObjKey` and
  `ObjGen` expose independently writable fields and carry no object marker.
- `poprako-obj-dept/src/oper.rs`: operations carry `K`, but metadata,
  allocation results, and upload confirmations lose it. `GenObjUrls<K>` can
  accept metadata returned for another kind. Lookup and deletion inputs are
  also ordinary strings.
- `poprako-obj-dept/src/rdb_impl.rs`: `decode_record` and `active_key`
  reconstruct `Dom` from a physical string solely to check duplicated fields.
- `poprako-obj-dept/src/prom.rs`: defer methods accept an independent topic
  string and an untyped key, permitting unrelated values to be combined.
- `poprako-obj-dept/src/actor/rdb_impl.rs`: dispatch parses a physical key
  before comparing task and current state. After validation, remote operations
  use the original stored path; the reconstructed domain value is discarded.
- Server mappings implement six grammars. Four delegate to
  `src/complex/image.rs`; chapter artwork and page artwork implement their own
  parsers. Their input structures still contain untyped business ID strings.

The current manifest in `src/part_impl/obj_dept.rs` is the source for all six
registrations: `PageSourceImage`, `PageArtworkImage`, `UserAvatar`, `TeamAvatar`,
`ComicCover`, and `ChapterArtwork`. The existing spec contains older manifest
syntax and path examples and must be reconciled during implementation.

## Target interface

| Value | Responsibility |
| --- | --- |
| `ObjKind` | Replace the bidirectional `KeyMap` contract with associated `Id`, `Dom`, and `Ext` types and one-way physical-key construction. |
| `ObjGen<K>` | Object-kind-specific identity and exact generation used for upload confirmation. |
| `ObjKey<K>` | Immutable identity, generation, and physical-key snapshot created together by ObjDept. |
| `ObjMeta<K>` | Metadata retaining the same object kind, including its typed extension. |
| `ObjSlot<K>` and `ObjAlloc<K>` | Upload capability and allocation result retaining their object kind. |
| `ObjPath` | Opaque physical location used by persistence and the pool; no business identity decoder. |
| Typed deferred obligation | Check or Delete data bound to `K` and its manifest topic before persistence. |

Use server-owned ID newtypes for the six relevant business identities. Their
validated constructors reuse the existing `BusinessIdKind` rules; they live
under `value` and do not depend on `part` or `part_impl`. Page identity and
parent chapter identity have distinct types. Keep unrelated business models
outside this migration; convert their strings at the ObjDept integration seam.

Image inputs retain `ImageExt`. Chapter artwork gains a validated extension
value instead of an arbitrary `String`. Validation occurs when constructing
these values, before allocation, rather than while interpreting a physical key.

Object operations accept the associated typed identity. Collection keys use
that identity too. `GenObjUrls<K>` accepts only `ObjMeta<K>`, and
`MarkObjUploaded<K>` accepts only `ObjGen<K>`. The public data interface must not
provide an unrestricted conversion between object markers.

Make key fields private and expose read accessors. Allocation constructs the
identity and physical snapshot together; normal callers cannot independently
replace either field. HTTP response conversion explicitly extracts scalar IDs
and versions. HTTP input validates the business ID and obtains the object kind
from its owning use case, never from a client-supplied marker or physical key.

## Physical key ownership

The server storage mapping defines each existing layout and produces an opaque
path from validated domain input and the allocated generation. ObjDept persists
that exact snapshot and passes it to the physical pool. Path layout belongs
beside `part_impl/obj_dept`, rather than in image business rules.

The pool consumes `ObjPath`. Its string representation is exposed only for
persistence, deterministic task encoding, storage SDK calls, and URL
construction. Public and signed URL parsing remains necessary for the URL
library. This refactor removes business-path reverse parsing, not all external
data decoding.

Stored paths remain authoritative locations. Old paths are not regenerated
using current domain data or a newer layout. This is necessary after a business
row is deleted, a parent changes, a generation is superseded, or an extension
changes. Current and historical task paths retain their exact bytes.

There is deliberately no second persisted copy of `Dom` and no new JSON
payload. Check/Delete require the structured ID and version already present in
the task plus its immutable physical location. They do not require recovering
the chapter ID or extension from that location.

## Persistence and task conversion

Keep raw Diesel record structures separate from public typed values. Read
conversions validate ID format, version range, extension, and active/detached
field combinations once, at the persistence seam. Only the generated module
for the concrete table restores its corresponding `ObjKey<K>` and metadata.
Explicit adapter restoration is distinct from ordinary caller construction.

Change task creation so callers provide a typed obligation and the generated
manifest binding supplies its topic. A topic string and arbitrary key must not
remain independently selectable in the ordinary defer interface. Preserve
batched insert and identity verification through typed Diesel expressions.

Global claiming still returns a raw heterogeneous task envelope. The existing
static topic dispatch chooses the typed handler, then validates and converts
that envelope once. Known operation strings become a Check/Delete enum before
execution. Unknown topics, invalid operations, invalid fields, and malformed
envelopes retain operator-repair handling.

Preserve deterministic task ID encoding, immutable obligation comparison,
visibility timing, retry behavior, claim UUID fencing, and same-generation
physical-key equality between a task and its active record. Preserve the
version watermark and revision compare-and-set checks independently of paths.

The trust contract must be explicit: Rust types prevent cross-kind use after
validated construction; they cannot prove the integrity of manually corrupted
database contents. Removing reverse parsing also removes the old check that a
path's embedded ID, version, and extension agree with separate columns. For
historical tasks whose business/object rows no longer exist, the persisted
snapshot is authoritative. Same-generation record/task mismatch detection
remains. This distinction must appear in the updated spec and regression tests;
it must not be described as compile-time verification of database bytes.

## Implementation sequence

1. **Define the typed interface.** Replace `KeyMap`, type all identity-bearing
   values and operations, restrict constructors, and add object-kind mismatch
   compile-fail cases with corresponding compiling examples. Add only the
   server ID and extension values needed by these six mappings.
2. **Update generated persistence and task creation.** Change macro templates
   for read, allocation, clear/delete, and defer operations together. Separate
   raw records from typed values, derive topics from the manifest, and retain
   typed Diesel projections and bounded batches. Do not edit generated schema.
3. **Update task execution and physical storage.** Restore typed task values at
   static dispatch, remove reverse mapping, and pass immutable opaque paths to
   the pool. Retain state classification, same-generation key comparison,
   revision checks, and claim fencing.
4. **Migrate the six server integrations.** Update allocation, confirmation,
   metadata/URL reads, publication clear, deletion, hierarchy sweeping, and
   comic archival. Update mock storage and fixtures at the same interfaces.
   Keep route payloads and business permission/transaction ownership stable.
5. **Remove obsolete contracts and reconcile documentation.** Delete all six
   reverse mappings, path parsers and their round-trip tests, the `Img = String`
   associated-type constraint, and untyped key/metadata/task producer escape
   paths. Keep independent ID, extension, hash, and URL validation. Update
   `specs/obj-dept.md` and affected current documentation to the final contract.
6. **Review and validate the complete slice.** Review hand-written and expanded
   macro contracts, compare lifecycle behavior with existing regressions, and
   run the checks below. Do not leave an untyped compatibility interface that
   production callers can continue using.

## Compatibility and migration

The default implementation preserves object-table and task-table columns,
stored paths, topic strings, task ID encoding, and HTTP JSON. This is a Rust
interface break across the workspace, with no planned R2 object copying or
database rewrite. Persistence serialization must remain compatible rather than
deriving a new on-disk representation from the new Rust wrappers.

Regression fixtures must cover records and pending tasks produced by the old
representation, including stale deletion obligations with no remaining owner.
There must be no online fallback that reverse-parses their physical paths.

If implementation identifies a genuinely necessary schema change, revise this
plan with concrete columns and existing-data handling before changing the
baseline. Follow the migration skill, regenerate with `just mgr-schema`, and
verify apply, revert-all, and apply on the authorized disposable CI database.

The working tree already contains substantial PageSource and business-ID work,
including modifications to the ObjDept mappings and older plans. Implement
against that working state, preserve unrelated changes, and re-read overlapping
files before edits.

## Acceptance and validation

- Compile-fail coverage rejects metadata and generation values from another
  object kind, wrong domain inputs, wrong typed IDs, and independent mutation
  or retagging of an object key. Include positive counterparts so a broken test
  fixture cannot masquerade as a type-safety result.
- Allocation regressions cover available-content reuse, pending reuse,
  extension changes, replacement, duplicate IDs, and all six exact path formats.
- Clear and Delete preserve generation semantics and atomically enqueue
  physical cleanup with business changes. Rollback leaves neither half applied.
- Actor regressions cover present/absent objects, stale and missing rows,
  future tasks, same-generation path mismatch, concurrent upload confirmation,
  CAS loss, claim replacement, retry, and operator repair.
- Historical paths survive round-trip persistence unchanged, including a
  stored opaque path that the removed business grammar cannot decode. Tests
  exercise the pool through a fake; no remote deletion is needed for validation.
- Macro/RDB tests verify typed table access and rollback, and the six server
  flows keep their current request and response contracts.
- Run `just fmt`, then `just fmt-check`, `just check`, and `just clippy` with
  their canonical workspace arguments. Keep Rust files strictly below 600 lines.
- Run affected crate and server tests, then `sh scripts/ci-test.sh`. Explicitly
  include feature-gated RDB tests with `rdb_impl` for ObjDept and `rdb` for the
  server against the configured disposable fixtures; default workspace tests
  alone are insufficient evidence for those modules.
- Search the final production slice for residual physical-key `reverse`
  implementations and parsers, raw string producer topics, and untyped
  identity-bearing values. Do not change or run custom repository linters
  outside the root validation policy. Any later commit must pass the complete
  pre-commit gate.
