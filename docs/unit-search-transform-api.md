# Chapter Unit search and transform API

## Search

`GET /api/v1/chapters/{chapter_id}/units/search` performs a preview search over
the visible Units in a Chapter. Both query parameters are required:

- `part`: `translated_text` or `proofread_text`;
- `phrase`: a case-sensitive literal substring. The server trims surrounding
  Unicode whitespace and accepts any non-empty result, including one Japanese
  character.

The response is `data: UnitInfoView[]`. Results follow Page index order and the
stored Unit order within each Page. Page indices are not added to the response.
Only the selected text field is searched. The implementation reads ordered
Pages in concurrent batches of 20 while preserving the final stable order.
At most 100 matching Units may exist across the Chapter. Exactly 100 matches
succeed; discovering the 101st returns `422 / Args(code 2)` with "Too many
matches", without a partial result or reading later Page batches.

Search uses the same Chapter membership or assignment visibility as Page Unit
listing. It is a preview only and does not create a replacement snapshot.

## Transform

`POST /api/v1/chapters/{chapter_id}/units/transform` accepts:

```json
{
  "part": "translated_text",
  "units": [
    {
      "unit_id": "unit-id",
      "transforms": [
        { "origin": "old text", "target": "new text" }
      ]
    }
  ]
}
```

The request accepts 1–100 unique Units and 1–20 transforms per Unit. Origins
must be unique and non-empty; targets may be empty. Matches are exact,
case-sensitive, and replace all occurrences. Every transform is evaluated
against the Unit's original text, so targets do not cascade into later pairs.
Overlapping original matches reject the complete request with `422`.

The client is expected to search first and submit only selected Units. Search
does not reserve their content: a pair whose origin is no longer present is
skipped. Missing or hidden Units are also skipped. A requested Unit that exists
under another Chapter rejects the complete request with `422`.

The operation runs in one serializable transaction and returns `204`, including
when every selected transform is a no-op. Translators may transform only
`translated_text`; proofreaders may transform only `proofread_text`. Successful
changes update the corresponding attribution, preserve `is_proofread` when
editing proofread text, update counters, and enter the normal Unit-edit workflow.
Serializable conflicts return `409` and require retrying the complete request.
