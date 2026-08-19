# Chapter Translation Port Format

## PopRaKo JSON

The PopRaKo export document is also the PopRaKo import document. The shared
DTOs are `ChapterTranslationPortView`, `PageTranslationPortView`, and
`UnitTranslationPortView`; their serialized field names are unchanged from
the export contract.

Import uses only page and unit indexes, coordinates, bubble state, translated
text, proofread text, and proofread state. Chapter, comic, page, and unit IDs,
titles, subtitles, and original translator/proofreader IDs are export
metadata. Imported units receive new IDs and are assigned to the importing
user subject to that user's translator and proofreader roles.

Page and unit indexes are zero-based. Page indexes must be complete and
unique, unit indexes must be non-negative and unique within a page, coordinates
must be finite, the document may contain at most 200 pages, and each page may
contain at most 100 units. The old project shape using `author`, `title`,
`image_filename`, `x`, `y`, `index_in_page`, `is_inbox`, `prooved_text`, and
`is_prooved` is not accepted.

## LabelPlus

LabelPlus import accepts one leading UTF-8 BOM, LF or CRLF line endings,
official and group-generated preamble spacing, and trailing spaces or tabs on
structure lines. Translation text is kept without an overall trim. Page files
are matched by page order; filenames are not matched to stored page images.
Export continues to use three-digit filenames from `000` through `199`.

Unit indexes must start at one and be unique within each page. Coordinates must
be finite, the bubble flag must be `1` or `2`, headers must be complete, the
document may contain at most 200 pages, and each page may contain at most 100
units. The service does not add a `label-plus` format alias, CR-only or
non-UTF-8 decoding, or format-specific HTTP error codes.

## Replacement semantics

Both formats fully replace the visible text boxes of every target page. An
empty imported page clears its target page. Existing visible units are soft
deleted in batches of at most 100, the unit order is read again, and imported
units are created in batches of at most 100 in one transaction. Hidden history
is retained. Page and chapter counters are written once from the original
state to the final state. Workflow records and stage transitions are part of
the same transaction, so a failure on any page rolls back the whole import.
