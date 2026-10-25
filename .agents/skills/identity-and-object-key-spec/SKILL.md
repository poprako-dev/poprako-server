---
name: identity-and-object-key-spec
description: Define and review business entity IDs, image object keys, source/artwork page identity, and Issue creation receipts in PopRaKo.
---

# Business identities and object keys

Use `src/value/business_id.rs` as the closed namespace registry and
`src/util.rs::next_business_id` for business entity creation. The canonical
form is `{snake_case_domain}--{11-character Base62 snowflake}`. The separator
is two hyphens. Keep String storage; never infer the domain from an arbitrary
string or add a second prefix table. `BusinessIdKind::is_valid_id` checks the
expected namespace and canonical u64 encoding, not record existence.

Business domains include user, team, member, member_invitation, workset,
comic, chapter, page_source, page_artwork, unit, issue, assignment,
assignment_invitation, announcement, comment, comic_archive, termbase, term,
system_mail, and chapter_workflow_record. Register additional domains before
introducing their generation paths. Technical task IDs, save-batch and lease
UUIDs, client local_id, qid, invitation codes, hashes, and versions retain
their independent formats. Image rows reuse the owner's business ID.

## Physical object keys

Use complete business IDs as path segments:

| Object | Canonical key |
| --- | --- |
| Source image | `page_source/{chapter_id}/{page_source_id}/{version}.{ext}` |
| Finished page image | `page_artwork/{chapter_id}/{page_artwork_id}/{version}.{ext}` |
| Chapter artwork file | `chapter_artwork/{chapter_id}/{version}.{ext}` |
| Comic cover | `comic_cover/{comic_id}/{version}.{ext}` |
| Team avatar | `team_avatar/{team_id}/{version}.{ext}` |
| User avatar | `user_avatar/{user_id}/{version}.{ext}` |

Construct physical locations once in the object-department adapter from
validated business IDs, typed extensions, and the allocated generation.
`ObjKind` has no reverse operation. `ObjKey<K>`, `ObjGen<K>`, metadata, upload
slots, and allocation results retain their object kind; identity-bearing
operations accept the associated typed ID. Keep their fields and constructors
from permitting independent mutation or implicit conversion between kinds.

Persist complete validated construction inputs in `f_domain JSONB` for object
rows and deferred tasks. Generate `ObjPath` only from the current one-way
mapping. Do not persist or accept arbitrary physical paths, legacy formats,
format aliases, or fallback decoders. Domain deserialization must reject
unknown or missing fields and validate every typed ID and extension. Check the
record owner and suffix against the restored domain before returning metadata.
Retain construction data for normal generation cleanup after owner deletion,
revision fencing, and manifest-bound task topics. Source images use
`page_source_image`. See repository-relative `specs/obj-dept.md`.

## Page and Issue contracts

`PageSource` is the source-page domain; `PageArtwork` is the finished-page
domain. Source API references use page_source_id, own resources use id, and
manifest collections use pages. Existing archive and translation exchange
wire field names stay stable through explicit serde names. Do not reinterpret
old IDs during import.

Manifests preserve identity only through explicit IDs. An absent ID creates
a new page even when hashes match; omitted existing IDs are deleted. Both
image allocation responses expose image_version. Identical available content
reuses its version without a slot; pending identical content requires a byte
length and re-signs the same version. Changed content starts a new generation.
Source keeps optimized images; finished pages expose original and thumbnail.

Issue saves accept exactly one of an existing id or a new local_id. Generate
the issue ID on the server and return created_issue_ids alongside page.
Record retry fingerprints and local-to-business ID mappings only after
commit. Charge mappings to the bounded receipt cache. Retries return the
same mapping and the current page after permission, review, and image guards.

## Validation and scope

Cover all registered ID kinds, wrong namespaces and malformed suffixes,
all six one-way key layouts and rejection of invalid persisted construction data, explicit manifest
identity, pending/available versions, rollback and stale confirmation, and
Issue retry mappings. Keep schema and OpenAPI regeneration under the existing
repository workflows. This skill does not authorize migration of historical
database rows, archived data, or remote object storage; such work requires
its own explicit scope.
