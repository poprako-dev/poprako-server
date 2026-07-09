# API HTTP Endpoint Plan

This plan defines the endpoint contract for wiring the active `src/usecase`
layer into Axum HTTP. It is a planning document only. Do not implement handlers
from this file until the endpoint contract is accepted.

## Global Contract

- Business routes live under `/api/v1`.
- Health lives at `/api/health`, is available in debug and release builds, and
  returns `204 No Content` only for localhost callers.
- Non-localhost `/api/health` requests return `404 Not Found` with no body.
- Swagger UI and OpenAPI JSON stay debug-only at `/api/swagger-ui` and
  `/api/openapi.json`.
- All business endpoints are protected by auth middleware except `/auth/register`
  and `/auth/login`.
- Auth middleware reads `authorization-token` cookie first, then
  `Authorization: Bearer <token>`.
- Register and login both set `authorization-token` as an HttpOnly cookie with
  value `Bearer {token}`.
- Successful responses with values return `HttpBody<T>`.
- Successful responses without values return `204 No Content` with no body.
- Errors return `HttpError`.
- Rename the legacy `HttpResponse<T>` body wrapper to `HttpBody<T>` before
  implementing handlers.
- CRUD create returns `201 Created + HttpBody<T>`.
- CRUD read/list returns `200 OK + HttpBody<T>`.
- `PUT`, `PATCH`, and `DELETE` success returns `204 No Content`.
- RPC success returns `204` when empty, `200` when returning a non-created value,
  and `201` when it creates a resource/relation.
- All list endpoints require `offset` and `limit`.
- HTTP enum values use `snake_case`.
- Query include options use `incl=...`.
- Query extra materialization options use `with=...`.
- Multiple query enum values use comma-separated strings.

## Route Shape Rules

- CRUD routes use REST resource shape.
- Instant business operations use RPC-style verb paths.
- Parent nesting is allowed only when the parent id is mandatory for the
  operation.
- Create routes use the created aggregate as the first-level path. Parent ids
  stay in the body and are the body-side source of truth for creates.
- Update body structs keep their `id` fields. HTTP handlers compare path ids
  with body ids and return `422 Unprocessable Entity` on mismatch.
- `PUT` means complete replacement/update.
- `PATCH` means partial update.
- If a current `Update*Data` is actually partial, rename it to `Patch*Data`
  before handler implementation.
- Deleting a whole child collection inside a parent scope may use nested
  `DELETE`.
- Translation exports use file-download semantics, not `HttpBody<String>`.

## Pre-Handler Fixes

These changes must happen before or during the handler implementation slice.

- Add/restore active HTTP result types:
  - `HttpBody<T>`
  - `HttpError`
  - `HttpResult<T>`
  - `Accept as _` support for valued responses
  - explicit no-body success support for `204`
- Rename `UpdateChapterInfoData` to `PatchChapterInfoData`.
- Rename matching `chapter::update_info` language to patch semantics, or keep the
  usecase name only if project naming rules allow it.
- Add pagination to `ListPageUnitInfosData` and the unit list repository/usecase
  path.
- Change `member::join_team` to return `MemberInfoVal`.
- Change `assignment_invitation::join` to return `AssignmentInfoVal`.
- Add request DTOs where current usecases accept loose values:
  - `MarkComicArchivedData {}`
  - `MarkSystemMailsReadData { ids }`
- Decide final download headers for translation export:
  - `Content-Type`
  - `Content-Disposition`
  - generated filename
- For user avatar reserve, either add `id` to the usecase target or make the
  handler reject `path user_id != token.user_id` before calling the current
  token-only usecase.

## Endpoint Summary

### Health

| Method | Path | Success | Usecase | Notes |
| --- | --- | --- | --- | --- |
| GET | `/api/health` | `204` | none | localhost only; non-localhost returns `404` |

### Auth

| Method | Path | Success | Usecase | Notes |
| --- | --- | --- | --- | --- |
| POST | `/api/v1/auth/register` | `201 HttpBody<RegisterVal>` | `auth::register` | public; sets cookie |
| POST | `/api/v1/auth/login` | `200 HttpBody<LoginVal>` | `auth::login` | public; sets cookie |

### Users

| Method | Path | Success | Usecase | Notes |
| --- | --- | --- | --- | --- |
| GET | `/api/v1/users/me` | `200 HttpBody<UserInfoVal>` | `user::get_info` | id comes from token |
| GET | `/api/v1/users/{user_id}` | `200 HttpBody<UserInfoVal>` | `user::get_info` | path id target |
| PUT | `/api/v1/users/{user_id}` | `204` | `user::update_info` | compare path id with body id |
| DELETE | `/api/v1/users/{user_id}` | `204` | `user::delete` | path id target |
| POST | `/api/v1/users/{user_id}/avatar/reserve` | `200 HttpBody<ReserveUserAvatarVal>` | `user::reserve_avatar` | RPC; path/token target check |
| POST | `/api/v1/users/{user_id}/avatar/mark-uploaded` | `204` | `user::mark_avatar_uploaded` | RPC |

### Teams

| Method | Path | Success | Usecase | Notes |
| --- | --- | --- | --- | --- |
| POST | `/api/v1/teams` | `201 HttpBody<TeamInfoVal>` | `team::create` | body carries create data |
| GET | `/api/v1/teams` | `200 HttpBody<Vec<TeamInfoVal>>` | `team::list_infos` | `user_id`, `offset`, `limit` |
| GET | `/api/v1/teams/{team_id}` | `200 HttpBody<TeamInfoVal>` | `team::get_info` | protected route, token unused |
| PUT | `/api/v1/teams/{team_id}` | `204` | `team::update_info` | compare path id with body id |
| DELETE | `/api/v1/teams/{team_id}` | `204` | `team::delete` | path id target |
| POST | `/api/v1/teams/{team_id}/avatar/reserve` | `200 HttpBody<ReserveTeamAvatarVal>` | `team::reserve_avatar` | RPC |
| POST | `/api/v1/teams/{team_id}/avatar/mark-uploaded` | `204` | `team::mark_avatar_uploaded` | RPC |

### Members

| Method | Path | Success | Usecase | Notes |
| --- | --- | --- | --- | --- |
| POST | `/api/v1/members` | `201 HttpBody<CreateMemberVal>` | `member::create` | body carries user/team ids |
| GET | `/api/v1/members` | `200 HttpBody<Vec<MemberInfoVal>>` | `member::list_infos` | `owner_id`, `team_id`, `fuzzy_nickname`, `role`, `incl`, `offset`, `limit` |
| GET | `/api/v1/members/me` | `200 HttpBody<Vec<MemberInfoVal>>` | `member::list_infos` | current user's memberships |
| PUT | `/api/v1/members/{member_id}/role` | `204` | `member::update_role` | compare path id with body id |
| DELETE | `/api/v1/members/{member_id}` | `204` | `member::delete` | path id target |
| POST | `/api/v1/members/join` | `201 HttpBody<MemberInfoVal>` | `member::join_team` | RPC; requires usecase return fix |

### Member Invitations

| Method | Path | Success | Usecase | Notes |
| --- | --- | --- | --- | --- |
| POST | `/api/v1/member-invitations` | `201 HttpBody<CreateMemberInvitationVal>` | `member_invitation::create` | body carries `team_id` |
| GET | `/api/v1/teams/{team_id}/member-invitations` | `200 HttpBody<Vec<MemberInvitationInfoVal>>` | `member_invitation::list_infos` | `pending`, `incl`, `offset`, `limit` |
| PUT | `/api/v1/member-invitations/{member_invitation_id}/role` | `204` | `member_invitation::update_info` | compare path id with body id |
| DELETE | `/api/v1/member-invitations/{member_invitation_id}` | `204` | `member_invitation::delete` | path id target |

### Worksets

| Method | Path | Success | Usecase | Notes |
| --- | --- | --- | --- | --- |
| POST | `/api/v1/worksets` | `201 HttpBody<CreateWorksetVal>` | `workset::create` | body carries `team_id` |
| GET | `/api/v1/teams/{team_id}/worksets` | `200 HttpBody<Vec<WorksetInfoVal>>` | `workset::list_infos` | `offset`, `limit` |
| GET | `/api/v1/worksets/{workset_id}` | `200 HttpBody<WorksetInfoVal>` | `workset::get_info` | path id target |
| PUT | `/api/v1/worksets/{workset_id}` | `204` | `workset::update_info` | compare path id with body id |
| DELETE | `/api/v1/worksets/{workset_id}` | `204` | `workset::delete` | path id target |

### Comics

| Method | Path | Success | Usecase | Notes |
| --- | --- | --- | --- | --- |
| POST | `/api/v1/comics` | `201 HttpBody<CreateComicVal>` | `comic::create` | body carries `workset_id` |
| GET | `/api/v1/worksets/{workset_id}/comics` | `200 HttpBody<Vec<ComicInfoVal>>` | `comic::list_infos` | `fuzzy_title`, `is_completed`, `incl`, `with`, `offset`, `limit` |
| GET | `/api/v1/comics/{comic_id}` | `200 HttpBody<ComicInfoVal>` | `comic::get_info` | path id target |
| PUT | `/api/v1/comics/{comic_id}` | `204` | `comic::update_info` | compare path id with body id |
| DELETE | `/api/v1/comics/{comic_id}` | `204` | `comic::delete` | path id target |
| POST | `/api/v1/comics/{comic_id}/cover/reserve` | `200 HttpBody<ReserveComicCoverVal>` | `comic::reserve_cover` | RPC |
| POST | `/api/v1/comics/{comic_id}/cover/mark-uploaded` | `204` | `comic::mark_cover_uploaded` | RPC |
| POST | `/api/v1/comics/{comic_id}/mark-archived` | `204` | `comic::mark_archived` | RPC |

### Chapters

| Method | Path | Success | Usecase | Notes |
| --- | --- | --- | --- | --- |
| POST | `/api/v1/chapters` | `201 HttpBody<CreateChapterVal>` | `chapter::create` | body carries `comic_id` |
| GET | `/api/v1/comics/{comic_id}/chapters` | `200 HttpBody<Vec<ChapterInfoVal>>` | `chapter::list_infos` | `incl`, `offset`, `limit` |
| GET | `/api/v1/comics/{comic_id}/chapters/pinned` | `200 HttpBody<Option<ChapterInfoVal>>` | `chapter::get_pinned` | preserves current optional return |
| GET | `/api/v1/chapters/{chapter_id}` | `200 HttpBody<ChapterInfoVal>` | `chapter::get_info` | path id target |
| PATCH | `/api/v1/chapters/{chapter_id}` | `204` | `chapter::update_info` | partial update; rename data to `PatchChapterInfoData` |
| DELETE | `/api/v1/chapters/{chapter_id}` | `204` | `chapter::delete` | path id target |
| POST | `/api/v1/chapters/{chapter_id}/stage/advance` | `204` | `chapter::update_stage` | RPC; compare path id with body id |
| POST | `/api/v1/chapters/{chapter_id}/translations/import` | `200 HttpBody<ChapterTranslationImportVal>` | `chapter_port::import` | RPC |
| GET | `/api/v1/chapters/{chapter_id}/translations/export` | file download | `chapter_port::export` or `export_label_plus` | `format=poprako,label_plus` |

### Pages

| Method | Path | Success | Usecase | Notes |
| --- | --- | --- | --- | --- |
| GET | `/api/v1/chapters/{chapter_id}/pages` | `200 HttpBody<Vec<PageInfoVal>>` | `page::list_infos` | `offset`, `limit` |
| DELETE | `/api/v1/chapters/{chapter_id}/pages` | `204` | `page::delete` | deletes all pages in chapter scope |
| POST | `/api/v1/chapters/{chapter_id}/pages/reserve` | `200 HttpBody<ReserveChapterPagesVal>` | `page::reserve_chapter_pages` | RPC; compare path id with body `chapter_id` |
| POST | `/api/v1/pages/{page_id}/image/reserve` | `200 HttpBody<ReservePageImageVal>` | `page::reserve_image` | RPC |
| POST | `/api/v1/pages/{page_id}/image/mark-uploaded` | `204` | `page::mark_image_uploaded` | RPC |

### Units

| Method | Path | Success | Usecase | Notes |
| --- | --- | --- | --- | --- |
| GET | `/api/v1/pages/{page_id}/units` | `200 HttpBody<ListPageUnitInfosVal>` | `unit::list_infos` | requires pagination prework |
| POST | `/api/v1/pages/{page_id}/units/save` | `200 HttpBody<SavePageUnitsVal>` | `unit::save_infos` | RPC; compare path id with body `page_id` and `diff.page_id` |

### Assignments

| Method | Path | Success | Usecase | Notes |
| --- | --- | --- | --- | --- |
| GET | `/api/v1/assignments` | `200 HttpBody<Vec<AssignmentInfoVal>>` | `assignment::list_infos` | `chapter_id`, `owner_id`, `role`, `incl`, `offset`, `limit` |
| PUT | `/api/v1/chapters/{chapter_id}/assignments/{user_id}/role` | `204` | `assignment::update_roles` | compare path ids with body `chapter_id` and `user_id` |
| DELETE | `/api/v1/assignments/{assignment_id}` | `204` | `assignment::delete` | path id target |
| POST | `/api/v1/assignments/join` | `201 HttpBody<AssignmentInfoVal>` | `assignment::join` | RPC |

### Assignment Invitations

| Method | Path | Success | Usecase | Notes |
| --- | --- | --- | --- | --- |
| POST | `/api/v1/assignment-invitations` | `201 HttpBody<CreateAssignmentInvitationVal>` | `assignment_invitation::create` | body carries `chapter_id` |
| GET | `/api/v1/chapters/{chapter_id}/assignment-invitations` | `200 HttpBody<Vec<AssignmentInvitationInfoVal>>` | `assignment_invitation::list_infos` | `pending`, `offset`, `limit` |
| DELETE | `/api/v1/assignment-invitations/{assignment_invitation_id}` | `204` | `assignment_invitation::delete` | path id target |
| POST | `/api/v1/assignment-invitations/join` | `201 HttpBody<AssignmentInfoVal>` | `assignment_invitation::join` | RPC; requires usecase return fix |

### System Mails

| Method | Path | Success | Usecase | Notes |
| --- | --- | --- | --- | --- |
| GET | `/api/v1/system-mails` | `200 HttpBody<Vec<SystemMailVal>>` | `system_mail::list_infos` | `read`, `offset`, `limit` |
| POST | `/api/v1/system-mails/mark-read` | `204` | `system_mail::mark_read` | RPC; body carries `ids` |

### Announcements

| Method | Path | Success | Usecase | Notes |
| --- | --- | --- | --- | --- |
| POST | `/api/v1/announcements` | `201 HttpBody<CreateAnnouncementVal>` | `announcement::create` | body carries `team_id` |
| GET | `/api/v1/teams/{team_id}/announcements` | `200 HttpBody<Vec<AnnouncementInfoVal>>` | `announcement::list_infos` | `incl`, `offset`, `limit` |

### Comments

| Method | Path | Success | Usecase | Notes |
| --- | --- | --- | --- | --- |
| POST | `/api/v1/comments` | `201 HttpBody<CreateCommentVal>` | `comment::create` | body carries `team_id` |
| GET | `/api/v1/teams/{team_id}/comments` | `200 HttpBody<Vec<CommentInfoVal>>` | `comment::list_infos` | `incl`, `offset`, `limit` |

## Implementation Order

1. Restore active `src/api/http` module skeleton, result body/error types, auth
   token constants, middleware, router, OpenAPI module, and health handler.
2. Implement `HttpBody<T>` and `204` no-body success support.
3. Implement auth middleware and cookie-setting register/login handlers.
4. Apply pre-handler DTO/usecase fixes listed above.
5. Implement low-risk read/list handlers first: teams, users, worksets.
6. Implement CRUD create/update/delete handlers with path/body id checks.
7. Implement upload and business RPC handlers.
8. Implement translation import/export with file-download headers.
9. Register all handlers in router and OpenAPI.
10. Run `cargo fmt`, targeted HTTP compile checks, and `cargo check`.
