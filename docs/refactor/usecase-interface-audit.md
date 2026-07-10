# poprako-s Capability Gap Audit For poprako-server Use Cases

This document is an implementation backlog, not an interface-shape diff.

Only list a gap when `poprako-s` exposes a use-case capability that the active
`poprako-server` code cannot currently provide. Do not list items merely because the
Rust implementation path, function name, DTO name, response wrapper, or return
shape is different.

## Explicit Non-Gaps

Do not assign agents to these items from this document:

- Assignment upsert behavior split across `assignment::update_roles` and
  `assignment::delete`.
  The Rust split is intentional.
- Rust upload version confirmation fields such as `avatar_version`,
  `cover_version`, and `image_version`.
  These are intentional Rust-side safety additions.
- Rust functions that return richer values than Go, when the original Go
  capability is still achievable.
- Go `UserStatsApp`.
  It is intentionally disabled in `poprako-s`.

## P0 Missing Whole Capability Areas

### Assignment Invitation

Go capability:

- `AssignmentInvApp.ListByChapter`
- `AssignmentInvApp.Create`
- `AssignmentInvApp.Delete`
- `AssignmentInvApp.JoinByInvCode`

Current Rust state:

- `src/usecase/assignment_invitation.rs` exists but is empty.
- No active `src/data/assignment_invitation.rs`.
- No active `src/model/assignment_invitation.rs`.
- No active `src/part/repo/assignment_invitation.rs`.
- No active `src/part/repo/step/assignment_invitation.rs`.

Required implementation:

- Add assignment invitation model/data/value types.
- Add repo steps and repo traits.
- Add repo mock behavior for usecase tests.
- Implement list by chapter with `chapter_id`, optional `pending`, and
  pagination.
- Implement create with `chapter_id`, `invitee_qid`, and role mask.
- Implement delete by invitation id.
- Implement join by invitation code:
  find pending invitation by code, validate current user identity and roles,
  create or update the assignment as required, and mark the invitation consumed
  in the same transaction.

Go references:

- `references/poprako-s/internal/app/assignment_invitation.go`
- `references/poprako-s/internal/app/val/assignment_invitation.go`
- `references/poprako-s/internal/app/impl/assignment_invitation.go`
- `references/poprako-s/internal/domain/model/aggr/assignment_inv.go`
- `references/poprako-s/internal/domain/model/query/assignment_inv.go`

### Chapter Import And Export

Go capability:

- `ChapterPortApp.Export`
- `ChapterPortApp.ExportLp`
- `ChapterPortApp.Import`

Current Rust state:

- No active `chapter_port` usecase module.
- No active export/import DTOs in `src/data`.
- No active equivalent of `ChapterExportVal`, `PageExportVal`,
  `UnitExportVal`, `ImportChapterArgs`, or `ImportChapterRes`.

Required implementation:

- Add export DTOs for chapter, pages, and units.
- Add import DTOs with `chapter_id`, `format`, and `content`.
- Implement JSON-safe export of one chapter with comic metadata, page data,
  image URLs, and units.
- Implement LabelPlus export.
- Implement import from supported formats into an existing chapter.
- Validate page count compatibility before applying imported unit text.
- Return imported page and unit counts.
- Keep import mutation transactional.

Go references:

- `references/poprako-s/internal/app/chapter_port.go`
- `references/poprako-s/internal/app/val/chapter_port.go`
- `references/poprako-s/internal/app/impl/chapter_port.go`
- `references/poprako-s/internal/domain/svc/chapter_export.go`
- `references/poprako-s/internal/domain/svc/chapter_import.go`

## P1 Missing Core Workflow Behavior

### Comic Create Must Create The First Chapter

Go capability:

- `ComicApp.Create` creates the comic.
- It also creates the first chapter in the same transaction.
- It increments the comic chapter counter.
- It touches comic last active time.
- It creates an initial reviewer assignment for the creator.
- It accepts optional `first_chapter_title`; empty means the default title.

Current Rust state:

- `src/usecase/comic.rs::create` creates only the comic.
- It increments the parent workset comic count.
- It does not create the first chapter.
- It does not create the creator's first chapter assignment.
- `CreateComicData` has no `first_chapter_title`.

Required implementation:

- Add `first_chapter_title: Option<String>` to `CreateComicData`, or choose the
  local Rust naming that matches nearby DTO style.
- Extend `comic::create` transaction to allocate the first chapter index.
- Create the first chapter with default subtitle behavior when no title is
  provided.
- Increment comic chapter count.
- Touch comic last active time.
- Create the creator assignment for the first chapter.
- Add positive and rollback tests for the full transaction.

Go references:

- `references/poprako-s/internal/app/val/comic.go`
- `references/poprako-s/internal/app/impl/comic.go`

### Member Join Team By Invitation Code

Go capability:

- `MemberApp.JoinTeam` lets an already registered user join a team using a
  member invitation code.

Current Rust state:

- `src/usecase/member.rs` has create/list/update/delete.
- No public usecase accepts a member invitation code for an existing user.
- Registration can consume an invitation code, but that does not cover the
  existing-user join-team capability.

Required implementation:

- Add join-team data with invitation code.
- Lock and validate the pending member invitation.
- Validate current user QQ id against the invitation target.
- Create the membership with invitation roles.
- Mark the invitation consumed in the same transaction.
- Reject duplicate membership.

Go references:

- `references/poprako-s/internal/app/member.go`
- `references/poprako-s/internal/app/val/member.go`
- `references/poprako-s/internal/app/impl/member.go`
- `references/poprako-s/internal/domain/svc/member_inv.go`

## P1 Missing List Filtering And Pagination Capabilities

### Comic List Filters

Go capability:

- List comics by workset.
- Filter by fuzzy title.
- Filter by pinned chapter workflow phase:
  upload, translate, proofread, typeset, review, publish.
- Apply offset/limit pagination.

Current Rust state:

- `ListComicInfosData` has `workset_id` and `with`.
- `comic::list_infos` only calls `ComicStep::list_infos_by_workset_id`.
- No fuzzy title filter.
- No workflow phase filters.
- No pagination.

Required implementation:

- Add a Rust comic list spec covering:
  `workset_id`, optional `fuzzy_title`, optional workflow phase filters,
  offset, and limit.
- Add DTO fields.
- Add repo step support.
- Add mock filtering and pagination behavior.
- Add usecase tests for every filter category and combined pagination.

Go references:

- `references/poprako-s/internal/app/val/comic.go`
- `references/poprako-s/internal/app/impl/comic.go`
- `references/poprako-s/internal/domain/model/query/comic.go`

### Workset List Pagination

Go capability:

- `WorksetApp.List` applies offset/limit pagination.

Current Rust state:

- `ListWorksetInfosData` has only `team_id`.
- `workset::list_infos` lists by team without pagination parameters.

Required implementation:

- Add offset/limit to `ListWorksetInfosData`.
- Add repo step support for paginated workset listing.
- Update mock behavior and tests.

Go references:

- `references/poprako-s/internal/app/val/workset.go`
- `references/poprako-s/internal/app/impl/workset.go`
- `references/poprako-s/internal/domain/model/query/workset.go`

### Member List Nickname Filter

Go capability:

- `MemberApp.ListByTeam` can filter by `user_nickname_keyword`.

Current Rust state:

- `ListMemberInfosData` has `user_nickname_keyword`.
- Its conversion currently rejects any request where
  `user_nickname_keyword.is_some()`.

Required implementation:

- Decide the Rust list spec field name for fuzzy nickname filtering.
- Stop rejecting valid team-list nickname filter input.
- Add repo step and mock filtering support.
- Add tests for positive match, no match, and invalid owner-list use.

Go references:

- `references/poprako-s/internal/app/val/member.go`
- `references/poprako-s/internal/app/impl/member.go`
- `references/poprako-s/internal/domain/model/query/member.go`

## P2 Missing Relation Include Capabilities

These are capability gaps because `poprako-s` lets callers request attached
related objects in the same usecase call. If the Rust API intentionally wants
separate fetches instead, record that decision elsewhere and remove the
corresponding item from this backlog.

### Comic Includes

Go capability:

- Comic list/detail can include:
  `workset`, `workset.team`, `creator`.

Current Rust state:

- `ComicInclOpt` exists but is not wired into `ListComicInfosData` or
  `get_info`.
- `ComicInfoVal` cannot carry included workset, team, or creator values.

Required implementation:

- Add include options to comic list and detail data.
- Extend model/list spec output to carry optional relations.
- Extend repo/mock assembly.
- Extend `ComicInfoVal`.

Go references:

- `references/poprako-s/internal/domain/model/enum/comic.go`
- `references/poprako-s/internal/app/val/comic.go`
- `references/poprako-s/internal/app/impl/comic.go`

### Chapter Includes

Go capability:

- Chapter list/detail can include:
  `comic`, `comic.workset`, `comic.workset.team`, `comic.creator`, `creator`.

Current Rust state:

- Chapter list/get data has no include options.
- `ChapterInfoVal` cannot carry included comic or creator values.

Required implementation:

- Add chapter include value type.
- Add include options to list/detail data.
- Extend repo/mock assembly.
- Extend `ChapterInfoVal`.

Go references:

- `references/poprako-s/internal/domain/model/enum/chapter.go`
- `references/poprako-s/internal/app/val/chapter.go`
- `references/poprako-s/internal/app/impl/chapter.go`

### Assignment Includes

Go capability:

- Assignment list by chapter/user can include:
  `user`, `chapter`, `chapter.comic`, `chapter.comic.workset`,
  `chapter.comic.workset.team`, `chapter.creator`, `chapter.comic.creator`.

Current Rust state:

- `ListAssignmentInfosData` has no include options.
- `AssignmentInfoVal` cannot carry included user or chapter values.

Required implementation:

- Add assignment include value type.
- Add include options to assignment list data and list spec.
- Extend repo/mock assembly.
- Extend `AssignmentInfoVal`.

Go references:

- `references/poprako-s/internal/domain/model/enum/assignment.go`
- `references/poprako-s/internal/app/val/assignment.go`
- `references/poprako-s/internal/app/impl/assignment.go`

### Member Includes

Go capability:

- Member list and get-by-user-team can include `user` and `team`.

Current Rust state:

- `MemberInclOpt` exists and is accepted by list data.
- `MemberInfoVal` cannot carry included user or team values.
- Public get-by-user-team usecase is missing.

Required implementation:

- Extend member model/list result to carry optional user and team values.
- Extend repo/mock assembly.
- Extend `MemberInfoVal`.
- Wire the same include behavior into list and get-by-user-team.

Go references:

- `references/poprako-s/internal/domain/model/enum/member.go`
- `references/poprako-s/internal/app/val/member.go`
- `references/poprako-s/internal/app/impl/member.go`

### Member Invitation Includes

Go capability:

- Member invitation list can include `invitor` and `invitee`.

Current Rust state:

- Member invitation list data has no include options.
- `MemberInvitationInfoVal` cannot carry invitor or invitee user values.

Required implementation:

- Add member invitation include value type.
- Add include options to list data and list spec.
- Extend repo/mock assembly.
- Extend `MemberInvitationInfoVal`.

Go references:

- `references/poprako-s/internal/domain/model/enum/member_inv.go`
- `references/poprako-s/internal/app/val/member_invitation.go`
- `references/poprako-s/internal/app/impl/member_invitation.go`

## Suggested Agent Split

Use separate agents with disjoint write scopes:

- Agent A: assignment invitation full chain.
- Agent B: chapter import/export full chain.
- Agent C: comic create full transaction and comic list filters.
- Agent D: member join-team, get-by-user-team, and member nickname filter.
- Agent E: include support for comic/chapter/assignment.
- Agent F: include support for member/member invitation.
- Agent G: workset pagination.

Before assigning include work, decide whether relation include support remains a
required Rust API capability or is intentionally replaced by separate fetches.
