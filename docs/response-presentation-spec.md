# Response presentation boundary

## Scope

This is the third architecture-review fix on `fix/architecture-review`,
following the shared Unit sequence and Chapter creation changes.
The implementation baseline is `872ad2ea`.

Comic, Chapter, and Assignment use cases currently coordinate object-ID
collection, URL snapshots, cover fallback resolution, and rendering.
The shared presentation module must own that complete protocol so callers
cannot accidentally omit a phase or hydrate related graphs separately.

## Required behavior

- Expose complete presentation operations under `usecase::internal::view`.
  Keep object-ID collectors and rendering snapshots private to that module.
- Remove the redundant entity presentation modules and call the complete
  operations directly.
- Keep authorization, instruction validation, and requested relationship
  reads in the domain use cases.
- Present a Comic list, its optional pinned Chapters, and their Assignments
  using one shared object snapshot. Preserve input Comic order and positional
  alignment of all three `ListComicInfosVal` collections, including absent
  Chapters and empty Assignment lists.
- Reuse the supplied pinned-Chapter snapshot when resolving cover fallbacks,
  including knowledge that a queried Comic has no pinned Chapter. Query only
  Comic IDs not covered by that snapshot.
- Preserve dedicated-cover precedence over pinned-Chapter first-page covers,
  origin and thumbnail URLs, nested relationships, and missing-object behavior.
- Deduplicate and sort object IDs per marker before loading. Do not perform
  object operations for empty marker batches. Preserve error propagation.
- Preserve the single-Assignment presentation contract: it needs no repository
  and does not load first-page cover fallbacks. Batch presentation retains its
  existing fallback behavior.
- Keep the shared object-URL helper available to Team and User presentation.
- Keep endpoint aggregation in existing Val types and reusable fragments in
  existing View types. Do not alter HTTP contracts, repository operations,
  permissions, transactions, migrations, or generated files.
- Do not introduce a generic rendering framework or new adapter traits.

## Validation

Exercise the complete presentation operations using existing repository and
object-department test doubles. Cover mixed-graph batching and list alignment,
known present and absent pinned-Chapter reuse, dedicated and fallback covers,
empty and partial graphs, sorted deduplication, and object-operation failures.
Retain the existing affected use-case tests.

Run the canonical formatting, workspace check, and Clippy commands; run the
affected tests and the full pre-commit validation gate. Review Standards and
Spec independently before committing on the existing fix branch.
