# Chapter review issues

Each Chapter has at most one current review. Issues are its Page-scoped details;
there is no separate review identity, history, append, or per-issue write API.
MangaProof must adapt its export to this PRK protocol. Its native metadata format
is background research only, not an accepted import format.

## Import

`POST /api/v1/chapters/{chapter_id}/issues/import`

```json
{"pages":[{"issues":[{"variant":"文字位置","layer_path":"0.1.0","rect":{"x_coord":0.1,"y_coord":0.2,"width":0.3,"height":0.1},"note":"向左移动"}]},{"issues":[]}]}
```

The `pages` array maps positionally to all current Pages in ascending Page order.
Its length must match exactly, including Pages without issues. Issue array order
assigns contiguous zero-based indexes. Every successful import replaces the whole
current Chapter review and renews all issue identities. All empty issue arrays
clear the Chapter. The existing HTTP body limit applies.

Only a current `REVIEWER` assignee can import. Administrator status alone does not
authorize it. Published Chapters reject imports. Import does not advance Review
or change Units or Unit statistics. The response's `data` contains
`imported_page_count` and `imported_issue_count`, including empty Pages.

`variant` is an open nonblank string. `layer_path` is an optional opaque nonblank
string; missing or null targets the composite Page. `rect` is independently
optional. Its coordinates use the whole Page, with top-left origin, finite values,
positive width and height, and all edges in `[0, 1]`. No MangaProof path syntax or
category list is enforced. `note` preserves original text, newlines, and emptiness.

## Read and lifecycle

`GET /api/v1/pages/{page_id}/issues` returns `data: IssueInfoView[]` in ascending
`index` order, with an empty array when no issues exist. The response includes
`id`, `page_id`, `index`, `variant`, nullable `layer_path` and `rect`, and `note`.
Access follows Page rules: team membership or a current Chapter assignment.
Missing resources and permission errors follow the existing Page conventions.

A newly confirmed artwork generation clears the review in the same transaction.
Repeated confirmation of the same generation preserves issues imported afterward.
Slot allocation, failed confirmations and stale generations do not clear issues.
An internal persisted confirmation baseline makes this work even after typesetting
has completed. Existing databases must run the explicit manual upgrade in
[chapter-issue-upgrade.md](chapter-issue-upgrade.md) before applying the baseline
migrations; that upgrade preserves available artwork confirmation. Fresh
baseline migrations only create the current tables.

Page deletion cascades issues; reordering retained Pages preserves ownership.
New Comic archives contain each Page's ordered `issues` array and remove the
active details after successful archival. Existing archive JSON stays unchanged.
Replacement, lifecycle cleanup, and archive reads use typed Diesel operations.
