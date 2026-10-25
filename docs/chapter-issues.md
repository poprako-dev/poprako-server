# Composite artwork pages and chapter review

Translation PageSources, composite artwork pages and ChapterArtwork are separate resources.
A translation PageSource is the source for Units. A `t_page_artwork` row is one finished
PSD composite, with a stable ID and its own Chapter sequence. Its image generation
lives in the `page_artwork_image` ObjDept (`t_page_artwork_image`). ChapterArtwork
remains the complete PSD ZIP; uploading either resource never constructs the other.
Clients extract and upload composites themselves.

Composite counts and order may differ from the translation source, including added
end pages. Filename (`raw_ident`) and hash are presentation/content metadata;
neither determines identity. Two pages may have identical names and images.

## Composite manifest and image lifecycle

`GET /api/v1/chapters/{chapter_id}/page-artworks` returns ordered page views with
ID, Chapter ID, zero-based index, raw identifier, nullable image generation and
raw and thumbnail URLs, upload state and timestamps. Image URLs are exposed as
`image_url` and `image_thumbnail_url`. Read permissions follow translation PageSources.
Unavailable generations have no read URLs.

`POST /api/v1/chapters/{chapter_id}/page-artworks/alloc` takes the complete final
manifest, in its desired order:

```json
{"pages":[{"page_artwork_id":"existing-id","raw_ident":"001.psd","image_hash":"BASE64_SHA256","ext":"png"},{"raw_ident":"credits.psd","image_hash":"BASE64_SHA256","new_byte_len":1024,"ext":"png"}]}
```

An existing ID must belong to this Chapter and occur once. Omitting the ID creates
a new page; omitting an old page deletes it, its Issues and its image metadata,
with physical object deletion queued transactionally. An empty manifest deletes
all composite pages. There is no automatic matching by filename, hash or index.

The response is `data.pages`, each containing `page_artwork_id`, `index`,
`image_version`, `image_hash`, `ext` and nullable upload `slot`. An available
identical hash and extension on the same page reuse its generation without an
upload slot. Otherwise `new_byte_len` is required and a new generation is allocated.
A pending generation is not reused. Existing page IDs survive reordering and
image changes, so their Issues survive too.

`POST /api/v1/page-artworks/{page_artwork_id}/image/alloc` changes only that page's image and
raw identifier. It accepts `raw_ident`, `image_hash`, required `new_byte_len` and
`ext`, returning the same per-page allocation result.

Upload each returned slot, then confirm through
`POST /api/v1/page-artworks/{page_artwork_id}/image/mark-uploaded` with
`{"image_version":1}`. Stale confirmations fail; repeated current confirmations
are valid. Allocation and confirmation never advance Chapter stages or clear
Issues. Image formats and size limits follow PageSource images.

A current TYPESETTER, REDRAWER or REVIEWER assignee, or team ADMIN, may maintain
composites. Published Chapters reject writes. Manifest, object changes and Comic
activity updates commit together or roll back together.

## Review import

`POST /api/v1/chapters/{chapter_id}/issues/import`

```json
{"pages":[{"page_artwork_id":"existing-id","issues":[{"variant":"文字位置","rect":{"x_coord":0.1,"y_coord":0.2,"width":0.3,"height":0.1},"note":"向左移动"}]}]}
```

Each target ID must identify a composite page in this Chapter and appear once.
Import does not require every composite page, nor any correspondence with source
PageSources. Every successful import replaces the entire current Chapter review and
renews Issue IDs. Missing targets lose their prior Issues. Empty `pages` or all
empty Issue arrays clear the review while retaining composite pages and images.
Issue array order assigns zero-based `index`; target array order does not reorder
composite pages. The existing HTTP body limit applies.

Only a current REVIEWER assignee may import; ADMIN alone is insufficient.
Published Chapters reject imports. Import does not advance Review or change Units.
The response includes `imported_page_count` (explicit targets including empty ones)
and `imported_issue_count`.

`variant` is any nonblank string. Obsolete or unknown issue fields, including
`layer_name` and `layer_path`, are rejected rather than silently discarded.

`rect` is independently optional, normalized to the entire composite with top-left
origin, finite values, positive size and all edges in `[0, 1]`. No category
enumeration is imposed. `note` preserves text, newlines and emptiness.

## Read, removal and archive

`GET /api/v1/chapters/{chapter_id}/issues` returns a flat `IssueInfoView[]`, ordered
by composite page index then Issue index. Each entry contains `id`,
`page_artwork_id`, `index`, `variant`, nullable `rect` and `note`.
Access follows PageSource read permissions. Empty reviews return an empty array.

Replacing or confirming either a composite image or ChapterArtwork ZIP preserves
Issues. Source PageSource deletion and reordering also preserve them. Removing a
composite page deletes its Issues; Chapter deletion cascades its composites and
Issues while scheduling object cleanup.

New Comic archives store `page_artworks` on each Chapter, including each page's
ID, index, raw identifier, timestamps and nested `issues`. Artworks are captured
even when the Chapter has no translation PageSources. Successful archival removes
active composites and Issues and queues image deletion in the same transaction.
Existing archive JSON is unchanged.

Fresh migrations create the business page table, its image ObjDept table, and
Issues referencing the business page. Databases retaining an older Issue layout
must explicitly migrate their data before these baseline migrations; application
startup never applies migrations. The separate ChapterArtwork confirmation
baseline in [chapter-issue-upgrade.md](chapter-issue-upgrade.md) still supports
idempotent ZIP confirmation and typesetting workflow, independently of Issues.

## Versioned reviewer editing

The editor uses versioned contracts without per-annotation workflow flags.
Import, export, editing, and newly generated archives contain no layer metadata.

- `GET /api/v1/page-artworks/{id}/issues` returns `page_artwork_id`, `review_key`,
  `revision`, `chapter_revision`, nullable `image_version` and ordered `issues`.
- `POST /api/v1/page-artworks/{id}/issues/save?save_id={uuid}` takes `review_key`,
  `revision`, nullable `image_version` and the complete ordered `issues` array.
  Each issue contains `variant`, optional `rect`, `note`, and exactly one identity:
  `id` for an existing annotation on this page, or `local_id` for a new annotation.
  Local IDs must be nonempty and unique within the batch. The server generates
  `issue--{11-character Base62 snowflake}` IDs and returns `{ page, created_issue_ids }`,
  where each mapping contains `local_id` and `issue_id`. Empty arrays clear the page.
  Other pages remain intact.
- `GET /api/v1/chapters/{id}/issues/variants` returns `review_key`, chapter
  `revision`, and actual `variants` usage counts across all composite pages.
- `POST /api/v1/chapters/{id}/issues/variants/rename` accepts that summary's
  `review_key` and `revision`, plus `from` and `to`. It atomically updates every
  reference and returns `summary` plus fresh affected `pages`. Renaming into an
  existing name or changing a fixed default is rejected.

Revision strings are opaque optimistic concurrency tokens. A mismatched page,
review generation or image version returns HTTP 409 (code 8). Import advances
review generation and all page revisions; ordinary saves advance only their page
revision and the chapter revision. Rename advances affected page revisions and
chapter revision. Artwork changes also invalidate chapter summary revisions.
Writes lock the chapter and verify current REVIEWER assignment and unpublished
state, including before returning a cached retry result. Reads use finished-page access.

Save receipts are process-local, bounded by `cache.issue_save_limit_mib` (16 MiB
by default) with `cache.issue_save_ttl_secs` (30 days by default). A receipt binds
its UUID to the actor, page and exact submitted payload. Successful retries return
the current document and the same created-ID mappings without replaying old contents.
The cache memory weight includes the mapping strings. Receipts are recorded only
after commit. Cache eviction, process restart and other replicas fall back to
version checks and stable identities; permanent exactly-once delivery is not
promised.

The frontend keeps fixed defaults (`居中`, `字体`, `字号`, `字重`, `修图`), chapter
usage and personal presets distinct. Personal presets are account-scoped browser
storage, shared across chapters on that browser. They are never a server category
entity, and chapter usage does not implicitly add a preset. Deleting a preset
requires no references in the current chapter, including unsaved drafts. A used
category rename changes this chapter and any matching personal preset; it does
not rename other chapters. Drafts are account/chapter/page-scoped IndexedDB data.
Saving, importing and renaming do not advance workflow. Completion first saves
all pages, then uses the existing chapter Review stage transition.

Existing databases must run `scripts/migrate-review-editor.sql` explicitly before
starting this server version. It adds the three revision columns transactionally
and preserves annotation data; it is safe to repeat. Fresh databases receive the
columns through the baseline migrations. Application startup does not migrate.
