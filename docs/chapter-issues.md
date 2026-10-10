# Composite artwork pages and chapter review

Translation Pages, composite artwork pages and ChapterArtwork are separate resources.
A translation Page is the source for Units. A `t_page_artwork` row is one finished
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
`image_url` and `image_thumbnail_url`. Read permissions follow translation Pages.
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
Issues. Image formats and size limits follow Page images.

A current TYPESETTER, REDRAWER or REVIEWER assignee, or team ADMIN, may maintain
composites. Published Chapters reject writes. Manifest, object changes and Comic
activity updates commit together or roll back together.

## Review import

`POST /api/v1/chapters/{chapter_id}/issues/import`

```json
{"pages":[{"page_artwork_id":"existing-id","issues":[{"variant":"文字位置","layer_name":"他们两个…","rect":{"x_coord":0.1,"y_coord":0.2,"width":0.3,"height":0.1},"note":"向左移动"}]}]}
```

Each target ID must identify a composite page in this Chapter and appear once.
Import does not require every composite page, nor any correspondence with source
Pages. Every successful import replaces the entire current Chapter review and
renews Issue IDs. Missing targets lose their prior Issues. Empty `pages` or all
empty Issue arrays clear the review while retaining composite pages and images.
Issue array order assigns zero-based `index`; target array order does not reorder
composite pages. The existing HTTP body limit applies.

Only a current REVIEWER assignee may import; ADMIN alone is insufficient.
Published Chapters reject imports. Import does not advance Review or change Units.
The response includes `imported_page_count` (explicit targets including empty ones)
and `imported_issue_count`.

`variant` is any nonblank string. `layer_name` is an optional human-readable
nonblank name copied from the reviewed PSD layer and preserved verbatim; absent
or null refers to the composite as a whole. It is display metadata, not an
identity or a qualified path; duplicate layer names are valid. Obsolete or unknown
issue fields are rejected rather than silently discarded. Import files must carry
the readable name, not a numeric PSD tree index.

`rect` is independently optional, normalized to the entire composite with top-left
origin, finite values, positive size and all edges in `[0, 1]`. No category
enumeration is imposed. `note` preserves text, newlines and emptiness.

## Read, removal and archive

`GET /api/v1/chapters/{chapter_id}/issues` returns a flat `IssueInfoView[]`, ordered
by composite page index then Issue index. Each entry contains `id`,
`page_artwork_id`, `index`, `variant`, nullable `layer_name`/`rect` and `note`.
Access follows Page read permissions. Empty reviews return an empty array.

Replacing or confirming either a composite image or ChapterArtwork ZIP preserves
Issues. Source Page deletion and reordering also preserve them. Removing a
composite page deletes its Issues; Chapter deletion cascades its composites and
Issues while scheduling object cleanup.

New Comic archives store `page_artworks` on each Chapter, including each page's
ID, index, raw identifier, timestamps and nested `issues`. Artworks are captured
even when the Chapter has no translation Pages. Successful archival removes
active composites and Issues and queues image deletion in the same transaction.
Existing archive JSON is unchanged.

Fresh migrations create the business page table, its image ObjDept table, and
Issues referencing the business page. Databases retaining an older Issue layout
must explicitly migrate their data before these baseline migrations; application
startup never applies migrations. The separate ChapterArtwork confirmation
baseline in [chapter-issue-upgrade.md](chapter-issue-upgrade.md) still supports
idempotent ZIP confirmation and typesetting workflow, independently of Issues.
