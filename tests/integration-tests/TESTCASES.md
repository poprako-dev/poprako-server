# Integration Test Cases

> **Maintenance rule:** every change to the integration tests under
> `tests/integration-tests/src/` MUST be reflected in this document in the same
> change. Add/remove/rename a case here whenever you touch the corresponding
> `*.ts`. Keep case IDs stable; do not renumber unless a case is removed.

This document enumerates every HTTP API integration test case driven by the
TypeScript suite at `tests/integration-tests/src/`. Cases are grouped by suite
file, in the order `src/main.ts` executes them.

## How to run

```text
# 1. PostgreSQL must be reachable at DATABASE_URL (see repo .env, port 3306 -> container 5432)
# 2. Apply migrations to db_poprako_r: `just mgr-run`
# 3. Build and start the Rust HTTP server:
cargo build && ./target/debug/poprako-r      # listens on 127.0.0.1:8888
# 4. Run the suite from the integration-tests project root:
cd tests/integration-tests && pnpm install && pnpm api
```

The suite calls `resetDatabase()` before and after the run, and asserts
`assertDatabaseIsSeedOnly()` at the end (see _Cleanup invariant_ below).

## Response conventions

- Success body (valued): `{ "code": 0, "data": <T> }` — `HttpBody<T>`.
- Success no-content: HTTP `204` with empty body — `NoContent`.
- Error body: `{ "code": <n>, "message": "..." }` — `HttpError`.

Error code mapping (from `src/api/http/result.rs`):

| code | HTTP status | Variant                    | Meaning                                            |
| ---- | ----------- | -------------------------- | -------------------------------------------------- |
| 1    | 500         | Unrecoverable              | Infra failure only (DB outage / pool / serde). MUST NOT be reachable by any client request — see invariant below. |
| 2    | 422         | `Expected::Args`           | Invalid arguments / query / not-found by id       |
| 3    | 401         | `Expected::Auth`           | Unauthenticated                                    |
| 4    | 403         | `Expected::Perm`           | Forbidden / permission denied                      |
| 7    | 422         | `HttpError::unprocessable` | Path id does not match body id                     |

### 5xx invariant

**No client request may produce a 5xx response.** Rate limiting returns `429`
(4xx) via `src/api/http/middleware/rate_limit.rs`. Code 1 (500) exists solely
for genuine infrastructure failures (DB connection lost, pool exhaustion,
serialization failure) and is concealed from clients. A leaked `DieselError::NotFound`
is mapped to `Expected::Args` (422, code 2) — not 500 — because a missing row at
the infra layer indicates the usecase forgot to handle the absent case, and the
client must still get a 4xx. Every smoke case below asserts an exact status +
code, so a regression that turns any 4xx into a 5xx (or into a different 4xx)
fails the suite.

## Seed state

`resetDatabase()` truncates every `t_*` table (RESTART IDENTITY CASCADE) then
inserts exactly:

| Table      | Seed row                                                                                         |
| ---------- | ------------------------------------------------------------------------------------------------ |
| `t_team`   | id `team-00000000-0000-0000-0000-000000000001`, name "PRTS"                                      |
| `t_user`   | id `user-00000000-0000-0000-0000-000000000001`, qid "123456", sadmin=true, argon2 hash           |
| `t_member` | id `member-00000000-0000-0000-0000-000000000001`, user+team above, all worker timestamps = NOW() |

### Cleanup invariant

`assertDatabaseIsSeedOnly()` runs in the `finally` block **before** the final
`resetDatabase()` (not after), so it verifies the suite actually self-cleans
rather than verifying that reset works. It requires row counts:

| Table                   | Expected rows |
| ----------------------- | ------------- |
| `t_team`                | 1             |
| `t_user`                | 1             |
| `t_member`              | 1             |
| every other `t_*` table | 0             |

Self-cleanup is performed by `runCleanup(context)` in `src/main.ts`, run in the
same `finally` block before the assert:

1. `DELETE /api/v1/worksets/{worksetId}` — cascades by FK to `t_comic`,
   `t_chapter`, `t_page`, `t_unit`, `t_assignment`, `t_assignment_invitation`.
2. `cleanupLeftoverRows({ commentId, announcementId })` — direct SQL delete of
   the `t_comment` and `t_announcement` rows (no HTTP delete endpoint exists for
   these resources) and `TRUNCATE t_local_message` (the prom outbox, populated
   by `RdbProm` for every image reservation; no worker drains it during tests).

`projectFlow` and `smokeAnnouncementRoutes` store the created `commentId` /
`announcementId` into `context.ids` so cleanup can target them by id.

## Shared context

Carried across suites via `TestContext`:

- `api`: `ApiClient` with base URL `API_BASE_URL` (default `http://127.0.0.1:8888`).
- `auth`: `{ token, userId } | null` — set by `auth` suite, used by later suites.
- `ids.teamId`: seed default team id.
- `ids.worksetId / comicId / chapterId / pageId / unitId`: populated by `projectFlow`, consumed by `allApiSmoke`.
- `ids.commentId`: populated by `projectFlow` (FLOW-07), consumed by cleanup.
- `ids.announcementId`: populated by `smokeAnnouncementRoutes`, consumed by cleanup.

---

## Suite 1 — `suites/health.ts` (`runHealthSuite`)

Verifies the unauthenticated boundary and the liveness endpoint.

| ID        | Method | Path               | Body | Expected    | Notes                        |
| --------- | ------ | ------------------ | ---- | ----------- | ---------------------------- |
| HEALTH-01 | GET    | `/api/health`      | —    | 204 no body | Liveness probe               |
| HEALTH-02 | GET    | `/api/v1/users/me` | —    | 401, code 3 | No bearer token → Auth error |

## Suite 2 — `suites/auth.ts` (`runAuthSuite`)

Super-admin login and self-profile. Sets `context.auth` for later suites.

| ID      | Method | Path                 | Body                                    | Expected                                      | Notes                                                           |
| ------- | ------ | -------------------- | --------------------------------------- | --------------------------------------------- | --------------------------------------------------------------- |
| AUTH-01 | POST   | `/api/v1/auth/login` | `{ qid: "123456", password: "123456" }` | 200, `data: { user_id, token }`               | `user_id` == seed user; `token` > 20 chars                      |
| AUTH-02 | GET    | `/api/v1/users/me`   | — (bearer set)                          | 200, `data: { id, nickname, qid, is_sadmin }` | `id` == AUTH-01 user_id; `qid` == "123456"; `is_sadmin` == true |

## Suite 3 — `suites/projectFlow.ts` (`runProjectFlowSuite`)

End-to-end creation of workset → comic → chapter → pages → units → comment.
Populates `context.ids` for the smoke suite.

| ID      | Method | Path                                             | Body (key fields)                                                    | Expected                                                                                         | Notes                                                      |
| ------- | ------ | ------------------------------------------------ | -------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ | ---------------------------------------------------------- |
| FLOW-01 | POST   | `/api/v1/worksets`                               | `{ name, description, team_id }`                                     | 201, `data: { id }`                                                                              | Stores `worksetId`                                         |
| FLOW-02 | POST   | `/api/v1/comics`                                 | `{ author, description, first_chapter_subtitle, title, workset_id }` | 201, `data: { id, chapter_id }`                                                                  | Stores `comicId`, `chapterId`                              |
| FLOW-03 | (sql)  | `grantChapterWorkerRoles(chapter_id, user_id)`   | —                                                                    | direct DB UPDATE                                                                                 | Assigns all chapter worker roles to the seed user          |
| FLOW-04 | POST   | `/api/v1/chapters/{chapterId}/pages/reserve`     | `{ chapter_id, file_ext: "jpg", page_count: 2 }`                     | 200, `data: { creations: [{ page_id, put_url, image_version }] }`                                | 2 creations; `put_url` starts with `http`; stores `pageId` |
| FLOW-05 | POST   | `/api/v1/pages/{pageId}/units/save`              | `{ diff: { opers: [1 unit], page_id }, page_id }`                    | 200, `data: { local_id_mappers, total_unit_count, translated_unit_count, proofread_unit_count }` | all counts == 1; stores `unitId` from mappers[0]           |
| FLOW-06 | GET    | `/api/v1/pages/{pageId}/units?offset=0&limit=20` | —                                                                    | 200, `data: { unit_infos, total_unit_count }`                                                    | `total_unit_count` == 1; `unit_infos[0].id` == `unitId`    |
| FLOW-07 | POST   | `/api/v1/comments`                               | `{ content, team_id }`                                               | 201, `data: { id }`                                                                              | stores `commentId` for cleanup                            |

## Suite 4 — `suites/allApiSmoke.ts` (`runAllApiSmokeSuite`)

Walks every HTTP resource group. Each case asserts an **exact** HTTP status and
`code` (no `<500` smoke): success cases assert `200`/`201`/`204`, error cases
assert the specific 4xx + code. Sub-suites run in this order. Requires
`context.ids` from `projectFlow`.

### 4.1 `smokeAuthRoutes`

| ID            | Method | Path                    | Body                                                | Expected      | Notes                                     |
| ------------- | ------ | ----------------------- | --------------------------------------------------- | ------------- | ----------------------------------------- |
| SMOKE-AUTH-01 | POST   | `/api/v1/auth/register` | `{ code: "missing-code", nickname, password, qid }` | 422, code 2   | invite code not found                     |
| SMOKE-AUTH-02 | POST   | `/api/v1/auth/logout`   | —                                                   | 204 no body   |                                           |

### 4.2 `smokeUserRoutes`

`userId` = `context.auth.userId`.

| ID            | Method | Path                                              | Body                    | Expected    | Notes                                   |
| ------------- | ------ | ------------------------------------------------- | ----------------------- | ----------- | --------------------------------------- |
| SMOKE-USER-01 | GET    | `/api/v1/users/{userId}`                          | —                       | 200, code 0 |                                         |
| SMOKE-USER-02 | PUT    | `/api/v1/users/{userId}`                          | `{ id, nickname, qid }` | 204         |                                         |
| SMOKE-USER-03 | POST   | `/api/v1/users/not-{userId}/avatar/reserve`       | `{ file_ext: "png" }`   | 403, code 4 | path user != token user                 |
| SMOKE-USER-04 | POST   | `/api/v1/users/not-{userId}/avatar/mark-uploaded` | `{ avatar_version: 1 }` | 403, code 4 | path user != token user                 |
| SMOKE-USER-05 | DELETE | `/api/v1/users/not-{userId}`                      | —                       | 403, code 4 | path user != token user                 |

### 4.3 `smokeTeamRoutes`

| ID            | Method | Path                                           | Body                                  | Expected    | Notes                             |
| ------------- | ------ | ---------------------------------------------- | ------------------------------------- | ----------- | --------------------------------- |
| SMOKE-TEAM-01 | GET    | `/api/v1/teams?offset=0&limit=20`              | —                                     | 200, code 0; len >= 1 | seed team present       |
| SMOKE-TEAM-02 | POST   | `/api/v1/teams`                                | `{ name: "Smoke Team", description }` | 201, code 0 | created; deleted at SMOKE-TEAM-07 |
| SMOKE-TEAM-03 | GET    | `/api/v1/teams/{team.id}`                      | —                                     | 200, code 0 |                                   |
| SMOKE-TEAM-04 | PUT    | `/api/v1/teams/{team.id}`                      | `{ id, name, description }`           | 204         |                                   |
| SMOKE-TEAM-05 | POST   | `/api/v1/teams/{team.id}/avatar/reserve`       | `{ file_ext: "png" }`                 | 200, code 0 | returns `ReserveVersionVal`        |
| SMOKE-TEAM-06 | POST   | `/api/v1/teams/{team.id}/avatar/mark-uploaded` | `{ avatar_version: 1 }`               | 204         |                                   |
| SMOKE-TEAM-07 | DELETE | `/api/v1/teams/{team.id}`                      | —                                     | 204         | cleans up SMOKE-TEAM-02           |

### 4.4 `smokeMemberRoutes`

| ID              | Method | Path                                                 | Body                               | Expected    | Notes                          |
| --------------- | ------ | ---------------------------------------------------- | ---------------------------------- | ----------- | ------------------------------ |
| SMOKE-MEMBER-01 | GET    | `/api/v1/members?team_id={teamId}&offset=0&limit=20` | —                                  | 200, code 0 |                                |
| SMOKE-MEMBER-02 | GET    | `/api/v1/members/me?offset=0&limit=20`               | —                                  | 200, code 0 |                                |
| SMOKE-MEMBER-03 | POST   | `/api/v1/members`                                    | `{ roles: 128, team_id, user_id }` | 422, code 2 | already a member               |
| SMOKE-MEMBER-04 | PUT    | `/api/v1/members/{defaultMemberId}/roles`            | `{ id, roles: 128 }`               | 204         |                                |
| SMOKE-MEMBER-05 | POST   | `/api/v1/members/join`                               | `{ code: "missing-code" }`         | 422, code 2 | invitation not found           |
| SMOKE-MEMBER-06 | DELETE | `/api/v1/members/missing-member`                     | —                                  | 422, code 2 | member not found               |

### 4.5 `smokeMemberInvitationRoutes`

| ID            | Method | Path                                                                       | Body                                 | Expected      | Notes                   |
| ------------- | ------ | -------------------------------------------------------------------------- | ------------------------------------ | ------------- | ----------------------- |
| SMOKE-MINV-01 | POST   | `/api/v1/member-invitations`                                               | `{ invitee_qid, roles: 2, team_id }` | 201, code 0   |                         |
| SMOKE-MINV-02 | GET    | `/api/v1/teams/{teamId}/member-invitations?pending=true&offset=0&limit=20` | —                                    | 200, code 0   |                         |
| SMOKE-MINV-03 | PUT    | `/api/v1/member-invitations/{id}/roles`                                    | `{ id, roles: 4 }`                   | 204           |                         |
| SMOKE-MINV-04 | DELETE | `/api/v1/member-invitations/{id}`                                          | —                                    | 204           | cleans up SMOKE-MINV-01 |

### 4.6 `smokeWorksetRoutes`

`worksetId` from `context.ids`.

| ID          | Method | Path                                                | Body                        | Expected    | Notes              |
| ----------- | ------ | --------------------------------------------------- | --------------------------- | ----------- | ------------------ |
| SMOKE-WS-01 | GET    | `/api/v1/teams/{teamId}/worksets?offset=0&limit=20` | —                           | 200, code 0 |                    |
| SMOKE-WS-02 | GET    | `/api/v1/worksets/{worksetId}`                      | —                           | 200, code 0 |                    |
| SMOKE-WS-03 | PUT    | `/api/v1/worksets/{worksetId}`                      | `{ id, name, description }` | 204         |                    |
| SMOKE-WS-04 | DELETE | `/api/v1/worksets/missing-workset`                  | —                           | 422, code 2 | workset not found  |

### 4.7 `smokeComicRoutes`

`comicId` from `context.ids`.

| ID             | Method | Path                                                    | Body                                 | Expected    | Notes                       |
| -------------- | ------ | ------------------------------------------------------- | ------------------------------------ | ----------- | --------------------------- |
| SMOKE-COMIC-01 | GET    | `/api/v1/worksets/{worksetId}/comics?offset=0&limit=20` | —                                    | 200, code 0 |                             |
| SMOKE-COMIC-02 | GET    | `/api/v1/comics/{comicId}`                              | —                                    | 200, code 0 |                             |
| SMOKE-COMIC-03 | PUT    | `/api/v1/comics/{comicId}`                              | `{ id, author, description, title }` | 204         |                             |
| SMOKE-COMIC-04 | POST   | `/api/v1/comics/{comicId}/cover/reserve`                | `{ file_ext: "png" }`                | 200, code 0 | returns `ReserveVersionVal` |
| SMOKE-COMIC-05 | POST   | `/api/v1/comics/{comicId}/cover/mark-uploaded`          | `{ cover_version: 1 }`               | 204         |                             |
| SMOKE-COMIC-06 | POST   | `/api/v1/comics/{comicId}/mark-completed`               | `{ is_completed: true }`             | 204         |                             |
| SMOKE-COMIC-07 | DELETE | `/api/v1/comics/missing-comic`                          | —                                    | 422, code 2 | comic not found             |

### 4.8 `smokeChapterRoutes`

`chapterId` from `context.ids`.

| ID            | Method | Path                                                                          | Body                                                          | Expected      | Notes                                   |
| ------------- | ------ | ----------------------------------------------------------------------------- | ------------------------------------------------------------- | ------------- | --------------------------------------- |
| SMOKE-CHAP-01 | POST   | `/api/v1/chapters`                                                            | `{ comic_id, subtitle: "Smoke Extra Chapter" }`               | 201, code 0   | extra chapter; deleted at SMOKE-CHAP-10 |
| SMOKE-CHAP-02 | GET    | `/api/v1/comics/{comicId}/chapters?offset=0&limit=20`                         | —                                                             | 200, code 0   |                                         |
| SMOKE-CHAP-03 | GET    | `/api/v1/comics/{comicId}/chapters/pinned`                                    | —                                                             | 200, code 0   |                                         |
| SMOKE-CHAP-04 | GET    | `/api/v1/chapters/{chapterId}`                                                | —                                                             | 200, code 0   |                                         |
| SMOKE-CHAP-05 | PATCH  | `/api/v1/chapters/{chapterId}`                                                | `{ id, pin: true, subtitle }`                                 | 204           |                                         |
| SMOKE-CHAP-06 | POST   | `/api/v1/chapters/{chapterId}/stage/advance`                                  | `{ id, oper: "advance", stage: "translate" }`                 | 204           | valid workflow advance                  |
| SMOKE-CHAP-07 | POST   | `/api/v1/chapters/{chapterId}/translations/import`                            | `{ content: "invalid-import-content", format: "label-plus" }` | 422, code 2   | invalid import content                  |
| SMOKE-CHAP-08 | GET    | `/api/v1/chapters/{chapterId}/translations/export?format=poprako`             | —                                                             | 200, raw body | raw export (not HttpBody envelope)      |
| SMOKE-CHAP-09 | GET    | `/api/v1/chapters/{chapterId}/translations/export/download?format=label-plus` | —                                                             | 200, raw body | raw file download (not HttpBody)        |
| SMOKE-CHAP-10 | DELETE | `/api/v1/chapters/{extraChapter.id}`                                          | —                                                             | 204           | cleans up SMOKE-CHAP-01                 |

### 4.9 `smokePageRoutes`

`pageId` from `context.ids`.

| ID            | Method | Path                                                   | Body                                             | Expected    | Notes                                              |
| ------------- | ------ | ------------------------------------------------------ | ------------------------------------------------ | ----------- | -------------------------------------------------- |
| SMOKE-PAGE-01 | GET    | `/api/v1/chapters/{chapterId}/pages?offset=0&limit=20` | —                                                | 200, code 0 |                                                    |
| SMOKE-PAGE-02 | POST   | `/api/v1/chapters/{chapterId}/pages/reserve`           | `{ chapter_id, file_ext: "jpg", page_count: 1 }` | 422, code 2 | chapter already has pages (error-chapter-pages-already-reserved) |
| SMOKE-PAGE-03 | POST   | `/api/v1/pages/{pageId}/image/reserve`                 | `{ file_ext: "jpg" }`                            | 200, code 0 | returns `ReserveVersionVal`                        |
| SMOKE-PAGE-04 | POST   | `/api/v1/pages/{pageId}/image/mark-uploaded`           | `{ image_version: 1 }`                           | 422, code 2 | image version mismatch                             |
| SMOKE-PAGE-05 | DELETE | `/api/v1/chapters/missing-chapter/pages`               | —                                                | 422, code 2 | chapter not found                                  |

### 4.10 `smokeUnitRoutes`

| ID            | Method | Path                                             | Body                                                      | Expected    | Notes                       |
| ------------- | ------ | ------------------------------------------------ | --------------------------------------------------------- | ----------- | --------------------------- |
| SMOKE-UNIT-01 | GET    | `/api/v1/pages/{pageId}/units?offset=0&limit=20` | —                                                         | 200, code 0 |                             |
| SMOKE-UNIT-02 | POST   | `/api/v1/pages/{pageId}/units/save`              | `{ diff: { opers: [], page_id: "wrong-page" }, page_id }` | 422, code 7 | path/body id mismatch       |

### 4.11 `smokeAssignmentRoutes`

`userId` = `context.auth.userId`.

| ID            | Method | Path                                                           | Body                                | Expected              | Notes                          |
| ------------- | ------ | -------------------------------------------------------------- | ----------------------------------- | --------------------- | ------------------------------ |
| SMOKE-ASGN-01 | GET    | `/api/v1/assignments?chapter_id={chapterId}&offset=0&limit=20` | —                                   | 200, code 0; len >= 1 | FLOW-03 granted roles          |
| SMOKE-ASGN-02 | POST   | `/api/v1/assignments/join`                                     | `{ chapter_id, roles: 2 }`          | 403, code 4           | role not assignable            |
| SMOKE-ASGN-03 | PUT    | `/api/v1/chapters/{chapterId}/assignments/{userId}/roles`      | `{ chapter_id, roles: 3, user_id }` | 403, code 4           | self-admin-role removal denied |
| SMOKE-ASGN-04 | DELETE | `/api/v1/assignments/missing-assignment`                       | —                                   | 422, code 2           | assignment not found           |

### 4.12 `smokeAssignmentInvitationRoutes`

| ID            | Method | Path                                                                                 | Body                                    | Expected      | Notes                          |
| ------------- | ------ | ------------------------------------------------------------------------------------ | --------------------------------------- | ------------- | ------------------------------ |
| SMOKE-AINV-01 | POST   | `/api/v1/assignment-invitations`                                                     | `{ chapter_id, invitee_qid, roles: 2 }` | 201, code 0   |                                |
| SMOKE-AINV-02 | GET    | `/api/v1/chapters/{chapterId}/assignment-invitations?pending=true&offset=0&limit=20` | —                                       | 200, code 0   |                                |
| SMOKE-AINV-03 | POST   | `/api/v1/assignment-invitations/join`                                                | `{ code }`                              | 422, code 2   | current user != invitee qid    |
| SMOKE-AINV-04 | DELETE | `/api/v1/assignment-invitations/{id}`                                                | —                                       | 204           | cleans up SMOKE-AINV-01        |

### 4.13 `smokeSystemMailRoutes`

| ID            | Method | Path                                     | Body          | Expected    | Notes |
| ------------- | ------ | ---------------------------------------- | ------------- | ----------- | ----- |
| SMOKE-MAIL-01 | GET    | `/api/v1/system-mails?offset=0&limit=20` | —             | 200, code 0 |       |
| SMOKE-MAIL-02 | POST   | `/api/v1/system-mails/mark-read`         | `{ ids: [] }` | 204         |       |

### 4.14 `smokeAnnouncementRoutes`

| ID           | Method | Path                                                     | Body                          | Expected      | Notes                                  |
| ------------ | ------ | -------------------------------------------------------- | ----------------------------- | ------------- | -------------------------------------- |
| SMOKE-ANN-01 | POST   | `/api/v1/announcements`                                  | `{ content, team_id, title }` | 201, code 0   | stores `announcementId` for cleanup    |
| SMOKE-ANN-02 | GET    | `/api/v1/teams/{teamId}/announcements?offset=0&limit=20` | —                             | 200, code 0   |                                        |

### 4.15 `smokeCommentRoutes`

| ID               | Method | Path                                                | Body | Expected    | Notes |
| ---------------- | ------ | --------------------------------------------------- | ---- | ----------- | ----- |
| SMOKE-COMMENT-01 | GET    | `/api/v1/teams/{teamId}/comments?offset=0&limit=20` | —    | 200, code 0 |       |

## Suite 5 — `suites/errorCases.ts` (`runErrorCaseSuite`)

Asserts the error envelope shape for representative bad inputs.

| ID     | Method | Path                                                                               | Body                                           | Expected    | Notes                                |
| ------ | ------ | ---------------------------------------------------------------------------------- | ---------------------------------------------- | ----------- | ------------------------------------ |
| ERR-01 | PUT    | `/api/v1/worksets/{worksetId}`                                                     | `{ id: "not-the-path-id", name, description }` | 422, code 7 | path/body id mismatch                |
| ERR-02 | GET    | `/api/v1/worksets/{worksetId}/comics?is_completed=true&stages=2&offset=0&limit=20` | —                                              | 422, code 2 | invalid query (incompatible filters) |
| ERR-03 | GET    | `/api/v1/teams/{defaultTeamId}-missing`                                            | —                                              | 422, code 2 | team not found by id                 |

---

## Bug fixes applied during this revision

These issues were surfaced by tightening the smoke assertions and the cleanup
invariant, and were fixed in the same change:

1. **5xx on `DELETE /api/v1/worksets/{id}`** — `ComicListSpec { limit: u64::MAX }`
   in `src/complex/workset.rs` and `AssignmentListSpec { limit: u64::MAX }` in
   `src/part_impl/effect_async/chapter.rs` overflowed to `LIMIT -1` when cast to
   `i64` (`u64::MAX as i64 == -1`), so Postgres rejected the cascade-list query
   with `diesel error: LIMIT must not be negative` → 500. Replaced with
   `i32::MAX as u64`, matching the 5 existing list-all sites. Client-triggered
   5xx eliminated.
2. **`DieselError::NotFound` mapped to 500** — `src/part_impl/rdb_core/result.rs`
   previously mapped a leaked NotFound to `Unrecoverable` (500). Now maps to
   `Expected::Args` (422, code 2, `error-not-found`) with a `tracing::warn!` so a
   forgotten `optional()` is still visible in logs. A client-supplied missing id
   can no longer produce 5xx via this path.
3. **Misleading page-reserve message** — `src/usecase/page.rs` reused
   `error-invalid-page-count` ("页面数量必须大于 0") for the "chapter already has
   pages" branch. New dedicated key `error-chapter-pages-already-reserved`.
4. **Missing i18n keys** — registered `error-not-found`, `error-comic-not-found`,
   `error-avatar-version-mismatch`, `error-cover-version-mismatch`, and
   `error-chapter-pages-already-reserved` in both `zh-CN/main.ftl` and
   `en-US/main.ftl` (previously rendered as
   `Unknown localization key: "..."`).
5. **Test bug — `stage/advance` oper** — `StageOper` is `Advance | Revert`
   (kebab-case); the smoke body sent `oper: "start"`, an invalid value that
   triggered an axum `JsonRejection` (422 with a non-`HttpError` body, no `code`
   field). Corrected to `oper: "advance"`, which advances the workflow and
   returns 204.
6. **Vacuous cleanup assert** — `assertDatabaseIsSeedOnly()` previously ran
   *after* `resetDatabase()` in the `finally` block, so it only verified that
   reset works. It now runs *before* reset, and the suite self-cleans via
   `runCleanup` (API delete workset cascade + SQL delete comment/announcement +
   truncate `t_local_message` outbox).
