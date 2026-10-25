# ObjDept Reliable Object Lifecycle Spec

## Scope

ObjDept owns the relation between one business object and its physical OSS
objects. A business object has one stable `id` and at most one newest logical
version. Upload checks and deletion are asynchronous, so it may temporarily
have zero to many physical versions.

`ObjGen<K>` identifies one object kind, validated business ID, and version.
`ObjKey<K>` also retains the immutable physical location generated from typed
domain input. Key fields are private; callers use read accessors. Physical
locations are never decoded into business identity. This slice supports only durable Check
and Delete work. Inventory scans and other recovery systems are out of scope.

The registered object kinds are `PageSourceImage`, `PageArtworkImage`,
`UserAvatar`, `TeamAvatar`, `ComicCover`, and `ChapterArtwork`.

Business IDs use `{snake_case_domain}--{11-character Base62 snowflake}`. Image
rows reuse their owning business ID; task IDs and other technical identities
keep their separate formats. The closed registry lives in
`src/value/business_id.rs`.

| Object kind | Physical key |
| --- | --- |
| PageSourceImage | `page_source/{chapter_id}/{page_source_id}/{version}.{ext}` |
| PageArtworkImage | `page_artwork/{chapter_id}/{page_artwork_id}/{version}.{ext}` |
| ChapterArtwork | `chapter_artwork/{chapter_id}/{version}.{ext}` |
| ComicCover | `comic_cover/{comic_id}/{version}.{ext}` |
| TeamAvatar | `team_avatar/{team_id}/{version}.{ext}` |
| UserAvatar | `user_avatar/{user_id}/{version}.{ext}` |

Every ID segment contains the complete prefixed ID. Typed construction validates
its expected domain and exact forward reconstruction. Old path formats are
not accepted. Historical database and OSS conversion is a separate operation.

Source and finished-page manifests retain identities only through explicit
IDs. An omitted ID creates a new page even when content hashes match; pages
omitted from the authoritative manifest are removed. Both allocations return
image_version: identical available content retains its generation and returns
no slot; identical pending content requires a byte length and re-signs the same
generation; new or changed content requires a byte length and a new generation.
Source views expose raw_ident, image_version, image_uploaded, original,
optimized, and thumbnail URLs. Finished pages expose original and thumbnail
URLs. Their independent image limits are page_source_image_limit_mib and
page_artwork_image_limit_mib, both defaulting to 25 MiB.

Source image allocation and confirmation require RAW_PROVIDER or REVIEWER.
Both lock and validate the unpublished chapter, permissions, page identity,
and generation within the transaction and update comic activity. Source
allocation preserves its deferred raw-provide advancement and Unit counters.

## Dependency direction

```text
poprako-rdb-core <- poprako-obj-dept <- poprako-server
```

`poprako-rdb-core` contains neutral RDB mechanics. `poprako-obj-dept` contains
the `ObjDeptPool`, `ObjDeptProm`, and `ObjDeptPromDefer` support traits, values, operations,
actor, typed Diesel glue, and declaration macros. `poprako-server` supplies the
private R2 support implementation, generated schema, business markers, concrete object manifest, concrete
RDB prom declaration, and the total `NormObjDept` composition.

No image-specific pool abstraction or server application port sits between
ObjDept and `ObjDeptPool`.

## Static declaration and dispatch

The server declares every supported object in one manifest:

```rust
objs_def! {
    PageSourceImage {
        table: t_page_source_image,
        topic: "page_source_image",
    },
}
```

`objs_def!` rejects duplicate marker, table, or topic values and
generates typed helpers against each concrete Diesel table. It also emits one
local callback manifest. `impl_obj_dept! { dept: NormObjDept, view: NormObjDeptView }` consumes that
manifest and generates every operation implementation plus the full static
topic dispatch. Its input never names a pool or prom implementation.

Adding a new object kind requires its marker, typed domain mapping, standard
Diesel table, and one manifest entry. `ObjDeptPool` and `ObjDeptProm` remain trait-constrained dependencies.
There is no runtime registry, erased RDB access, boxed future, or dynamic trait
dispatch in the object-kind path.

## Diesel safety

Migrations are the schema source and Diesel generates `schema.rs`. Generated
code directly references the concrete table and all required columns. A
missing table, missing column, incompatible SQL type, or wrong nullability
therefore fails compilation.

Object tables contain `f_id`, `f_version`, nullable `f_domain JSONB`, `f_is_uploaded`, `f_hash`,
`f_ext`, and timestamps. `t_obj_prom_task` contains the durable task envelope,
including `f_topic`. These tables deliberately avoid business foreign keys and
business lifecycle checks; mutable policy stays in application code.

Raw SQL, runtime table names, `QueryableByName`, and schema-erasing RDB traits
are forbidden in ObjDept.

## Operations and values

`ObjDept<K, C>` drives:

- `ListObjMetas<K>` for newest persisted metadata;
- `GenObjUrls<K>` for origin and optional thumbnail URLs derived from
  supplied metadata versions;
- `GenObjSlot<K>` for allocation and a signed write capability;
- `GenObjSlots<K>` for batch allocation and signed write capabilities;
- `ClearObjs<K>` for clearing current files while their owning business
  entities remain active;
- `DeleteObjs<K>` for ending object lifecycles together with permanently
  deleted business entities.

Operation structures expose ordinary associated constructors and keep their
phantom marker fields private. Callers import and invoke the real operation
type directly so rust-analyzer navigation reaches its definition; no operation
construction macro is used. `MarkObjUploaded<K>` supports both independent Run and transaction-scoped Step. It returns `bool`, bound as
`marked` and handled at the immediate use-case boundary.

`ObjKind` defines associated `Id`, `Dom`, and `Ext` types and one-way physical
key construction. `ObjSlotSpec<K>` carries typed domain input, `hash`,
`content_type`, and `byte_len`. Server object-facing identities use validated
String-backed ID newtypes and the existing `BusinessIdKind` namespace registry.
Images use `ImageExt`; chapter artwork uses `ArtworkExt`.
Allocation returns `ObjAlloc<K>::Avail { key }` for identical available
content or `ObjAlloc<K>::Upload { slot }` for new or resumed pending content.
Both retain the exact selected generation; callers use `key()` without a
metadata reload and `into_slot()` when only the upload capability is needed.
Batch allocation returns one allocation for every requested object ID,
including available content. Duplicate requested IDs are rejected.
`ObjMeta<K>` carries typed `key`, `is_avail`, `hash`, and `ext`.
Allocation results and upload slots retain the same marker. Metadata and URL
batches use the associated identity as their map key. Lookup, clear, and delete
inputs use typed IDs; upload confirmation accepts `ObjGen<K>`.
`GenObjUrls<K>` accepts only metadata for that same object kind. Availability is an
optimistic read decision, not proof that remote bytes were verified. Component failures
use `ObjDeptError` and `ObjDeptRest`.
`ObjUrls` carries optional origin, optimized, and thumbnail URLs. The current
R2 pool provides a thumbnail; the optional field permits a future pool or
object kind without that capability.

`ClearObjs` makes the current files unavailable and guarantees that later
reservations cannot reuse an earlier generation. `DeleteObjs` ends the object
lifecycle and is valid only when the owning business identifiers will never be
reused for that object kind. Both record Delete work in the caller-owned
transaction before business mutation commits; neither performs remote I/O in
that transaction. A future object kind that permits business-ID reuse must use
`ClearObjs` or introduce a monotonic identity epoch.

## Canonical construction data and persistence

Object tables and durable tasks persist `f_domain` as JSONB containing the
complete typed construction input. Source and finished-page images retain the
chapter ID, page ID, and extension; other kinds retain their owner ID and
extension. There is no persisted physical-path column.

`ObjKind::Dom` supports strict serialization and deserialization. Server domain
structures reject unknown fields, require all fields, and deserialize IDs and
extensions through their validated types. `ObjKey::from_record` rejects a
mismatch between the domain owner and the record ID. Metadata restoration also
rejects a suffix column that differs from the domain suffix. Detached rows
retain only their owner identity and version watermark.

Every physical path is generated by the current one-way mapping from these
validated values and the selected generation. Neither arbitrary stored paths
nor previous path formats are accepted. Task IDs encode the structured domain;
there is no old encoding fallback. Same-generation task and active-record
locations must agree before remote I/O.

Deferred cleanup retains its complete construction input even after its owner
is deleted. This supports ordinary object replacement and deletion on a fresh
database. Existing data requires a separately authorized offline conversion;
the running service contains no compatibility or repair path.

## Pool and durable tasks

`ObjDeptPool` directly exposes `gen_slot`, `gen_urls`, `has`, and `del` against
opaque `ObjPath` values. It is
payload-neutral. Callers select original, optimized, and thumbnail URLs through `ObjUrlSpec`. Artwork exports select only the original URL; the R2 adapter does not infer image renditions from the file extension.

`ObjDeptProm` owns reset, global claim, completion, retry, and operator transitions.
`ObjDeptPromDefer<C>` records single or batched Check and Delete work inside the
caller-owned transaction. Producers supply `ObjKey<K>`; `ObjRegistration` binds
its topic from the manifest, so callers cannot select a topic independently.
The RDB adapter is generated by `rdb_obj_dept_prom!`
against `t_obj_prom_task` with ordinary typed Diesel queries. A batch uses
one bulk upsert and one bulk identity/status read rather than one operation per
object. A matching conflict locks the existing row through the caller transaction,
so successful completion cannot remove it before identity validation.

Task identity is deterministic. A repeated defer is accepted only when its
immutable obligation matches. Conflicting data is an error. Deduplication lasts
while the task exists; successful completion deletes the task, allowing the
same obligation to be deferred again. Operator tasks require repair.

Each atomic claim generates a fresh UUID execution credential in PostgreSQL.
Completion, retry, and operator transitions match the task ID, processing
status, and exact credential. Retry and operator transitions clear the credential;
expired processing work returns to pending and clears it too. An old execution
cannot mutate a task recreated with the same deterministic ID. Remote failures
have no attempt limit. Object version watermarks are independent of task history.

## Single actor

The generic actor lifecycle is always compiled. Only its typed RDB handler
support lives under `actor::rdb_impl` and follows the `rdb_impl` feature.

`ObjDeptActor::new(prom, handler)` stores injected dependencies without starting
background work. `run_detach(self)` starts one supervisor and returns its
non-cloneable `ObjDeptActorDesc`. Claim and maintenance loops run under that
supervisor; claiming is global across topics and tasks are processed serially. Topic dispatch is a generated static match whose
branches validate raw task fields, decode the Check/Delete discriminator, and
restore the corresponding typed key before calling their handlers. The actor owns whether future topic-level
parallelism is introduced; composition never creates one actor per topic.

The composition root owns the descriptor separately from `NormObjDept`.
Shutdown calls `cancel`, then consuming `join` waits for exit and returns any
supervisor task error. Dropping the descriptor requests cancellation; dropping
a department clone does not affect the actor. Each attempt
has a deadline, and each remote call has a shorter timeout. Physical `has` and
`del` calls occur outside RDB transactions.

## Check and Delete behavior

A Check becomes visible after the write capability expires plus grace time.

- The exact current version runs `has`, including a generation optimistically
  marked uploaded by the client-facing endpoint.
- Presence leaves or marks that exact generation available.
- Absence resets that exact generation to unavailable and completes the Check.
- Missing, stale, or retired state deletes the physical key idempotently.
- Future version or invalid state moves to operator repair.

Every Check snapshots the row's update revision before remote I/O. Its final
write compares the object ID, version, and that revision. A concurrent mark of
the same generation therefore causes a retry instead of being overwritten;
a newer generation is likewise never changed. Every lost compare-and-set
reloads the typed row and classifies the new state before deciding whether to
complete, retry, delete, or require repair.

Object presence is deliberately accepted as upload evidence. Check does not
read or compare remote content hashes because that verification cost is
currently incompatible with the upload-throughput requirement.

Delete work removes a missing, stale, or retired physical version
idempotently. A task targeting a pending, verified, or future current state
requires operator repair. Unresolved deletion work is never discarded because
of retry count.

## Business integration

PageSource, User, Team, and Comic tables no longer carry OSS lifecycle fields. Their
use cases select a logical object by business row id plus compile-time marker.
Reserve flows validate policy, then run `GenObjSlot` or `GenObjSlots` in the
owner transaction.
Read flows combine business data with explicit ObjDept metadata or URLs.
Delete and cascade flows run `DeleteObjs` in the same transaction as
business-row mutation. Workflows that keep their business entities but clear
current files run `ClearObjs`.

The mark-uploaded endpoints optimistically set the submitted exact current
ObjDept generation to uploaded. Origin and supported thumbnail URLs are
therefore immediately readable after a successful mark. This client claim does
not prove that the PUT succeeded: the delayed Check performs `has` for the same
generation and resets its uploaded flag when the object is absent. Exact-version
compare-and-set prevents that correction from changing a newer generation.

Both page-image reservation paths defer the chapter raw-provide advancement
task in the same transaction. That handler loads the chapter's complete page
set and every `PageSourceImage` metadata row. It advances the stage only when all
images are available; unavailable images leave the task available for a later
attempt without consuming a failure budget.

## Acceptance

- dependency direction remains `rdb-core -> obj-dept -> server`;
- main constructs and injects the pool and durable-task adapters, stores the
  total `NormObjDept` in Harn, and separately owns the actor descriptor;
- every supported object is registered only through `objs_def!`;
- generated Rust identifiers do not use a double-underscore prefix;
- generated code remains fully Diesel type checked;
- multi-object lifecycle and task persistence use bounded batch statements;
- Check/Delete work is isolated in `t_obj_prom_task`;
- no image-specific pool or old unified-prom image handler remains;
- all image and chapter artwork flows use ObjDept operations;
- all page-image upload paths can trigger automatic stage advancement;
- full formatting, all-feature compilation, tests, custom linters, and schema
  regeneration pass.

## Chapter artwork integration

Each chapter owns one `ChapterArtwork` slot in `t_chapter_artwork`, registered
in the same typed manifest and served by the existing object actor. Artwork
keys use `chapter_artwork/{chapter_id}/{version}.{ext}`. The frontend packs
and uploads the complete file directly; the API never transfers or parses its
bytes. Uploads accept safe file extensions and canonical Base64 SHA-256
identities, with an exact signed byte length bounded by
`[artwork].chapter_artwork_limit_mib` (MiB, default 512).

`alloc_artwork` returns the current version even when deduplication produces
no PUT slot. New content replaces the current version immediately. Export
requires an available current object and returns its original public URL.
Artwork does not request optimized or thumbnail image URLs.

`mark_artwork_uploaded` locks the owning chapter and uses the transaction-scoped
`MarkObjUploaded` operation. Availability, completion of `TypesetRedraw`, and
its `ArtworkUpload` workflow record commit together. The workflow-completion
event is emitted after commit only on a real transition. Repeated confirms do
not repeat completion; a stale generation cannot confirm the current object.
The delayed existence check can revoke availability without reverting the
business stage.

Publishing clears the artwork alongside page images in the publication
transaction. Published chapters reject allocation and confirmation. Chapter
deletion, hierarchy sweeping, and comic archival delete artwork associations
and defer physical deletion through the existing durable task mechanism.
