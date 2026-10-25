# Ownership and copy audit

Scope: Unit sequence reconstruction, shared Chapter creation, request-scoped
response presentation, and the Model/Shape ownership migration.
This audit covers payload copies, including `to_owned`, `to_string`, and container
copies, rather than only calls spelled `clone`. It does not claim that every
persisted projection in the application shares a global owner.

## Unit sequence reconstruction

The shared algorithm accepts non-Clone records, borrows record IDs while building
its index, and permutes the original slice in place. RDB page groups borrow input
page IDs and record their positions. Ranking queries borrow candidate page IDs;
the keyset cursor borrows the final retained row until the next query completes.
No string copy is needed for ordering, validation, grouping, or pagination.

Mock ordering and counts operate on references into the locked state. Detail
selection copies only selected records; search copies only returned IDs.

Retained production copies:

| Source → owner | Lifetime and necessity |
| --- | --- |
| Borrowed Unit edit fields → Mock state | The stored text, IDs, and coordinates must survive the edit instruction. Mutating a later instruction cannot change stored data. |
| Mock Unit state → returned detail, order, or search result | The result survives releasing the state lock and must remain a snapshot after later writes. Only fields in the selected output are copied. |
| Static corruption message → owned error | This constructs the error's string; it does not duplicate a successful record or sorting buffer. |

RDB duplicate page IDs retain first-occurrence output behavior. Mock duplicate
page IDs retain repeated output behavior; this cleanup does not change either
existing contract.

## Chapter creation

Comic, Chapter, Assignment, and workflow-record entries are explicit
`shape::write` types with `Shape<'a>` names and borrowed foreign IDs.
Existing model/token IDs are borrowed through the operation's await; generated
IDs and subtitles move into their Shapes. `Cow` is not used. Diesel insert records borrow
the entries. The shared creation helper and Comic creation make no ID copies.
Workflow helpers also borrow actor IDs instead of accepting an owned temporary.

Retained production copies:

| Source → owner | Lifetime and necessity |
| --- | --- |
| Borrowed entry → Mock persisted Comic/Chapter/Assignment | The state remains alive after the entry is dropped. It owns generated fields as well as foreign IDs. |
| Mock persisted projection → create result | The returned projection remains independently usable after subsequent Mock writes. State and response cannot borrow each other across the released transaction guard. |
| Borrowed workflow entry → Mock history | The state retains ID, actor, and payload after the input entry slice is destroyed. |
| Assignment user ID → owned history payload | The returned Assignment and the persisted event both retain the ID. The event's payload has independent persistence/serialization lifetime. |

ChapterPatch now moves its owned subtitle from the request. Export/background-job
snapshots retain their independently owned data. SQL payload
encoding produces an SQL parameter representation, not a redundant foreign-ID
copy.

## Response presentation

`ComicListData` owns the request's models. Pinned chapters are owned once in a
vector; comic alignment and assignment associations use integer positions.
Temporary lookup keys borrow these models and expire before rendering moves the
models into DTOs. Missing, known-absent, and unqueried pinned chapters remain
distinct during fallback loading.

Discovery and rendering use the same traversal order. The cover queue owns URL
results by occurrence, eliminating owned comic-to-page grouping keys. Fallback
page IDs move out of repository models. The temporary sorted metadata query
vector copies references from the ordered occurrence vector, not ID strings:
query deduplication and rendering occurrence order require separate sequences.

`ObjUrlView` has no Clone implementation. Single-use URLs move from `Url` into
`String` without copying the text buffer, then move into the DTO. Only repeated
consumers allocate an `Arc<String>`; intermediate consumers increment its reference
count and the final consumer takes the cached handle. PageSource URL usage counts
include only occurrences without dedicated cover metadata. An existing metadata
record whose URL fields are absent still suppresses fallback, preserving the
previous behavior.

Retained production copies:

| Source → owner | Lifetime and necessity |
| --- | --- |
| Cached repeated URL handle → earlier DTO occurrence | Each occurrence outlives hydration independently. `Arc::clone` adds ownership without copying bytes; the final occurrence moves the original handle. |
| Ordered occurrence references → sorted query references | These are pointer/length pairs. Sorting and deduplication must not destroy the order/multiplicity consumed by rendering. |
| Object adapter results → cache | Adapter-owned map keys and URL strings are moved. Existing object-port response construction is not replaced with a global interning layer. |

Present JSON URL values remain strings and absent fields remain omitted.
Comic, Team, and User schema fields explicitly retain their string schema; no ownership enum is exposed in HTTP or OpenAPI.
There is no leaked storage, unsafe code, global pool, or custom response serializer.

## Test-only ownership

Fixtures sometimes clone models to create independent repeated inclusions or
store owned parent entries and child projections in one fixture. Borrowing another field of
the same movable fixture would require self-referential storage. Mock ports copy
query IDs into call logs so assertions can run after the query input disappears.
These test copies are separate from production presentation and ordering.

Regression coverage includes non-Clone sequence records; tombstones, corruption,
multiple pages and cursor boundaries; independent Mock snapshots; borrowed workflow-entry identifiers; real RDB creation/rollback; pinned relation alignment,
partial snapshots and absence; metadata batching and fallback precedence; and
single/repeated URL pointer identity with unchanged string serialization.


## Owned Models and borrowed plans

All Model storage now owns its data; trim getters borrow normalized slices
without replacing or duplicating the original strings. DTO conversions move
strings and vectors. Adapters use the same getters, with Mock copying into
its independently owned state. Empty optional term comments and termbase
descriptions become absent; profile descriptions retain explicit presence.
Unit content and review notes retain their original whitespace.

| Shape | Owner and avoided copy |
| --- | --- |
| Comic/Chapter/Assignment/workflow entries | Loaded projections and tokens retain foreign IDs through persistence; generated fields move into the Shape. |
| UnitWritePlanShape and UnitWriteChangeShape | Submitted edits and current sequence retain text/IDs; sparse writes borrow them. New edits remain owned. |
| PageSourceRawIdentsReplShape | Manifest/page data retain strings; the vector of reference pairs moves into the Shape. |
| TermEntryShape and TermReplShape | Owned imports and persisted terms retain source, targets, and IDs during bulk writes. Reference vectors are allocated without string copies. |
| TermAppendReplShape | Existing and imported terms retain targets; merged comments require a new owned string. |

Single-term commands remain owned Models. Normalization iterators allocate no
string copies; Diesel batch rows may need vectors of string references. Append
plans also retain case-folded keys for deduplication and reference vectors for
stable ordering. These allocations implement distinct indexes/sequences and
are not eliminated by the ownership migration.
