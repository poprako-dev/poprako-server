# poprako-s → poprako-server Use Case Refactoring Checklist

> Each aggregate must be rebuilt in dependency order: **domain model → query trait → infra query → use cases → API handler**.
>
> Checkboxes `[ ]` are a single task; `[-]` means sub-tasks follow.
>
> `(ref)` points to the Go source file under `references/poprako-s/internal/`.

---

## 0. Already Implemented (Baseline)

These aggregates have full domain model, query trait, infra query (entity + impl + mock),
and are wired into `Harness`. Only their **remaining use cases** are listed below.

| # | Component | Status |
|---|-----------|--------|
| 0.1 | Domain: `Event`, `DomainError`, `EffectSink`, `Transactional` foundation | ✅ |
| 0.2 | Harness (`Harness`, `TestHarness`), `impl_forward_ref!` macro | ✅ |
| 0.3 | Domain aggr: `user` | ✅ |
| 0.4 | Domain aggr: `team` | ✅ |
| 0.5 | Domain aggr: `member` | ✅ |
| 0.6 | Domain aggr: `member_invitation` | ✅ |
| 0.7 | Domain aggr: `system_mail` | ✅ |
| 0.8 | Domain query: `user`, `team`, `member`, `member_invitation`, `system_mail` | ✅ |
| 0.9 | Infra query: all 5 entities + impls + memory mock, Diesel schema | ✅ |
| 0.10 | Infra query: `RdbQuery`, `MemoryMockQuery` | ✅ |
| 0.11 | Infra external: `JwtIssuer`, `OssImagePool` | ✅ |
| 0.12 | Use case: `User::sign_up_user` | ✅ |
| 0.13 | API: HTTP server, middleware, router scaffolding, `/auth/register` | ✅ |

---

## 1. User (`user` aggregate)

> Go refs: `app/user.go`, `app/impl/user.go` (416 loc), `app/val/user.go`, `app/res/res.go`

### 1.1 — Domain Model & Query (incomplete items only)

- [x] `UserAggr`, `UserForm`, `UserToken` — ✅
- [x] `UserQuery` + `UserRepoTransactional` traits — ✅
- [x] Infra entity (`UserRow`, `UserCredentialRow`) + Diesel impl + mock — ✅
- [ ] Add `UserPatch` / `UserUpdate` aggregate types for `Update` use case
  → (put semantics: nil for nullable fields means write SQL NULL)
- [ ] Add `avatar_key` / `avatar_uploaded_at` fields to `UserForm` (for `ResvAvatar` / `MarkAvatarUploaded`)

### 1.2 — Use Cases

- [x] `sign_up_user` — ✅
- [ ] `get_info(cx, id) → UserVal`
  - (ref: `app/impl/user.go` `GetInfo`)
  - Also serves as "GetMyInfo" when `id == currUid`
  - Fetch `UserAggr` + optional stats; no auth check for self-read
- [ ] `login(cx, args: LoginArgs) → LoginRes`
  - (ref: `app/impl/user.go` `Login`)
  - Validate qid + password via bcrypt; return signed JWT token
  - Emit event on successful login (for `TouchLastActive`)
- [ ] `register(cx, args: RegArgs) → RegRes`
  - (ref: `app/impl/user.go` `Register`)
  - NOTE: `sign_up_user` in Rust is a fused "register + join" flow; Go `Register` is standalone (no team join).
    Decide whether to keep separate or fold into `sign_up_user`.
- [ ] `update(cx, args: UpdArgs) → None`
  - (ref: `app/impl/user.go` `Update`)
  - Put semantics: overwrite nickname, avatar fields
  - Only the owner can update their own profile
- [ ] `resv_avatar(cx, curr_uid, args: ResvAvatarArgs) → ResvAvatarRes`
  - (ref: `app/impl/user.go` `ResvAvatar`)
  - Generate OSS signed PUT URL for avatar upload
- [ ] `mark_avatar_uploaded(cx, curr_uid) → None`
  - (ref: `app/impl/user.go` `MarkAvatarUploaded`)
  - Set `avatar_uploaded_at` = now; confirm OSS object exists
- [ ] `touch_last_active(cx, id) → None`
  - (ref: `app/impl/user.go` `TouchLastActive`)
  - Update `last_active_at` timestamp (no auth required)

### 1.3 — API Handlers

- [x] `GET    /users/:user_id` → `get_info`
- [x] `GET    /users/me` → `get_info` (current token user alias)
- [x] `POST   /auth/login` → `login`
- [x] `POST   /auth/register` → `register` / `sign_up_user`
- [x] `PUT    /users/me` → `update`
- [x] `POST   /users/:user_id/avatar` → `resv_avatar` (requires `user_id` = token user)
- [x] `POST   /users/:user_id/avatar/confirm` → `mark_avatar_uploaded` (requires `user_id` = token user)

---

## 2. Team (`team` aggregate)

> Go refs: `app/team.go`, `app/impl/team.go` (346 loc), `app/val/team.go`

### 2.1 — Domain Model & Query

- [x] `TeamAggr`, `TeamForm` — ✅
- [x] `TeamQuery` + `TeamRepoTransactional` traits — ✅
- [x] Infra entity + Diesel impl + mock — ✅
- [ ] Add `TeamPatch` / `TeamUpdate` for `Update` use case
- [ ] Add avatar fields (`avatar_key`, `avatar_uploaded_at`) to `TeamForm`

### 2.2 — Use Cases

- [ ] `create(cx, curr_uid, args: CreArgs) → CreRes`
  - (ref: `app/impl/team.go` `Create`)
  - Create team; creator auto-becomes admin member
  - Transaction: insert team + insert member
- [ ] `get_info(cx, id) → TeamVal`
  - (ref: `app/impl/team.go` `GetInfo`)
  - Fetch by id; no auth required (public info)
- [ ] `list(cx, curr_uid, args: ListArgs) → [TeamVal]`
  - (ref: `app/impl/team.go` `List`)
  - Paginated list of all teams (public)
- [ ] `list_by_user(cx, user_id, args: ListArgs) → [TeamVal]`
  - (ref: `app/impl/team.go` `ListByUser`)
  - Paginated list of teams where user is a member; also serves as "ListMyTeams"
- [ ] `update(cx, curr_uid, args: UpdArgs) → None`
  - (ref: `app/impl/team.go` `Update`)
  - Admin-only; put semantics for name, description, avatar
- [ ] `resv_avatar(cx, curr_uid, args: ResvAvatarArgs) → ResvAvatarRes`
  - (ref: `app/impl/team.go` `ResvAvatar`)
  - Admin-only; reserve OSS PUT URL for team avatar
- [ ] `mark_avatar_uploaded(cx, curr_uid, team_id) → None`
  - (ref: `app/impl/team.go` `MarkAvatarUploaded`)
  - Admin-only; confirm avatar upload

### 2.3 — API Handlers

- [ ] `POST   /teams` → `create`
- [ ] `GET    /teams/:team_id` → `get_info`
- [ ] `GET    /teams` → `list`
- [ ] `GET    /users/:user_id/teams` → `list_by_user`
- [ ] `GET    /me/teams` → `list_by_user` (self alias)
- [ ] `PUT    /teams/:team_id` → `update`
- [ ] `POST   /teams/:team_id/avatar/reserve` → `resv_avatar`
- [ ] `POST   /teams/:team_id/avatar/uploaded` → `mark_avatar_uploaded`

---

## 3. Member (`member` aggregate)

> Go refs: `app/member.go`, `app/impl/member.go` (381 loc), `app/val/member.go`

### 3.1 — Domain Model & Query

- [x] `MemberAggr`, `MemberForm` — ✅
- [x] `MemberQuery` + `MemberRepoTransactional` traits — ✅
- [x] Infra entity + Diesel impl + mock — ✅

### 3.2 — Use Cases

- [ ] `create(cx, curr_uid, args: CreateArgs) → CreateRes`
  - (ref: `app/impl/member.go` `Create`)
  - Admin-only; add a user to team with specified roles
  - Must check user exists; must not duplicate membership
- [ ] `list_by_team(cx, curr_uid, args: ListByTeamArgs) → [MemberVal]`
  - (ref: `app/impl/member.go` `ListByTeam`)
  - `curr_uid` must be a member of the team
  - Paginated; includes user info (preloaded)
- [ ] `list_mine(cx, curr_uid, args: ListMyMemberArgs) → [MemberVal]`
  - (ref: `app/impl/member.go` `ListMine`)
  - List all memberships of `curr_uid` across teams
- [ ] `update_role(cx, curr_uid, args: RoleUpdArgs) → None`
  - (ref: `app/impl/member.go` `UpdateRole`)
  - Admin-only; put semantics for role mask (7 role timestamps)
  - If role mask is zero → redirect to `delete`
- [ ] `delete(cx, curr_uid, member_id) → None`
  - (ref: `app/impl/member.go` `Delete`)
  - Admin-only; hard delete the membership row
- [ ] `get_by_user_team_id(cx, curr_uid, args: GetByUserTeamIdArgs) → MemberVal`
  - (ref: `app/impl/member.go` `GetByUserTeamId`)
  - `curr_uid` must be a member of the target team
  - Optional includes (e.g. `MemberInclUser` to preload user info)
- [ ] `join_team(cx, curr_uid, args: JoinTeamArgs) → None`
  - (ref: `app/impl/member.go` `JoinTeam`)
  - Join a team by invitation code; consume the invitation
  - NOTE: overlaps with `sign_up_user` in Rust; consider deduplication

### 3.3 — API Handlers

- [ ] `POST   /teams/:team_id/members` → `create`
- [ ] `GET    /teams/:team_id/members` → `list_by_team`
- [ ] `GET    /me/members` → `list_mine`
- [ ] `PUT    /teams/:team_id/members/:member_id/role` → `update_role`
- [ ] `DELETE /teams/:team_id/members/:member_id` → `delete`
- [ ] `GET    /teams/:team_id/members/by-user/:user_id` → `get_by_user_team_id`
- [ ] `POST   /members/join` → `join_team`

---

## 4. Member Invitation (`member_invitation` aggregate)

> Go refs: `app/member_invitation.go`, `app/impl/member_invitation.go` (240 loc), `app/val/member_invitation.go`

### 4.1 — Domain Model & Query

- [x] `MemberInvitationAggr`, `MemberInvitationForm` — ✅
- [x] `MemberInvitationQuery` + `MemberInvitationRepoTransactional` traits — ✅
- [x] Infra entity + Diesel impl + mock — ✅

### 4.2 — Use Cases

- [ ] `list(cx, curr_uid, args: ListArgs) → [MemberInvVal]`
  - (ref: `app/impl/member_invitation.go` `List`)
  - Admin-only; list pending (or all) invitations for a team; paginated
- [ ] `create(cx, curr_uid, args: CreateArgs) → CreateRes`
  - (ref: `app/impl/member_invitation.go` `Create`)
  - Admin-only; create invitation with invitee_qid, roles, unique code
  - Checks: invitee must not already be a member, no duplicate pending invite for same team+qid
- [ ] `update(cx, curr_uid, args: UpdArgs) → None`
  - (ref: `app/impl/member_invitation.go` `Update`)
  - Admin-only; update role mask of a pending invitation (put semantics)
- [ ] `delete(cx, curr_uid, inv_id) → None`
  - (ref: `app/impl/member_invitation.go` `Delete`)
  - Admin-only; delete a pending invitation

### 4.3 — API Handlers

- [ ] `GET    /teams/:team_id/invitations` → `list`
- [ ] `POST   /teams/:team_id/invitations` → `create`
- [ ] `PUT    /teams/:team_id/invitations/:inv_id` → `update`
- [ ] `DELETE /teams/:team_id/invitations/:inv_id` → `delete`

---

## 5. System Mail (`system_mail` aggregate)

> Go refs: `app/sys_mail.go`, `app/impl/sys_mail.go` (78 loc), `app/val/sys_mail.go`

### 5.1 — Domain Model & Query

- [x] `SystemMailAggr` — ✅
- [x] `SystemMailQuery` + `SystemMailRepoTransactional` traits — ✅
- [x] Infra entity + Diesel impl + mock — ✅

### 5.2 — Use Cases

- [ ] `list(cx, curr_uid, args: ListArgs) → [SysMailVal]`
  - (ref: `app/impl/sys_mail.go` `List`)
  - Returns unread system mails for `curr_uid`; paginated; ordered by `created_at DESC`
  - No auth beyond being the owner
- [ ] `mark_read(cx, curr_uid, id) → None`
  - (ref: `app/impl/sys_mail.go` `MarkRead`)
  - Mark one system mail as read for `curr_uid`

### 5.3 — API Handlers

- [ ] `GET    /me/mails` → `list`
- [ ] `POST   /me/mails/:mail_id/read` → `mark_read`

---

## 6. Workset (`workset` aggregate) — **new: domain model needed**

> Go refs: `app/workset.go`, `app/impl/workset.go` (276 loc), `app/val/workset.go`
> Go aggr: `domain/model/aggr/workset.go`, Go enum: `domain/model/enum/workset.go`

### 6.1 — Domain Model

- [-] Set up `WorksetAggr`, `WorksetForm`, `WorksetPatch`, `WorksetVal` structs
  - (ref: `aggr/workset.go`)
  - Fields: `id`, `team_id`, `name`, `desc`, `active`, `comic_count`
  - `WorksetForm` for create; `WorksetPatch` for update (put semantics)
  - ID generation via cuid2
- [ ] Add `WorksetVal` (read-model value object, used in API responses)
- [ ] Define `WorksetQueryKind` enum (All / Active / ByTeam / ById)
- [ ] Add `src/domain/model/value/workset.rs` if extra shared value types needed

### 6.2 — Domain Query Trait

- [-] Create `src/domain/query/workset.rs` with `WorksetQuery` + `WorksetRepoTransactional`
  - `WorksetQuery`: `get_by_id`, `list_by_team` (paginated, active filter)
  - `WorksetRepoTransactional`: `create`, `update`, `delete`, `inc_comic_count`, `dec_comic_count`
- [ ] Register `WorksetRepoForward` forward-trait in harness macro

### 6.3 — Infra Query

- [ ] Create `src/infra/query/entity/workset.rs` — `WorksetRow` with Diesel `Queryable`
  - Table: `workset_table` (hard-delete, so no `deleted_at`)
- [ ] Add `workset_table` to `schema.rs` (via `diesel print-schema` — ensure migration exists)
- [ ] Create `src/infra/query/workset.rs` — Diesel impl of `WorksetQuery` + `WorksetRepoTransactional`
- [ ] Create `src/infra/query/memory_mock/workset.rs` — in-memory mock for tests
- [ ] Register mock in `MemoryMockQuery` builder/seed/snapshot methods

### 6.4 — Use Cases

- [ ] `list(cx, curr_uid, args: ListArgs) → [WorksetVal]`
  - (ref: `app/impl/workset.go` `List`)
  - `curr_uid` must be a member of the team; returns active worksets for team
  - Paginated; preload team info
- [ ] `create(cx, curr_uid, args: CreateArgs) → CreatedRes`
  - (ref: `app/impl/workset.go` `Create`)
  - `curr_uid` must be an admin of the team
  - Validate: name unique within team? (check Go ref)
- [ ] `update(cx, curr_uid, args: UpdArgs) → None`
  - (ref: `app/impl/workset.go` `Update`)
  - Admin-only; put semantics for name + description
- [ ] `delete(cx, curr_uid, workset_id) → None`
  - (ref: `app/impl/workset.go` `Delete`)
  - Admin-only; hard delete (no cascade — Go ref `cascade_util.go` deletes comics→chapters→pages→units manually)
  - ⚠️ **Cascade**: before deleting workset, delete all comics under it + their chapters/pages/units/assignments

### 6.5 — API Handlers

- [ ] `GET    /teams/:team_id/worksets` → `list`
- [ ] `POST   /teams/:team_id/worksets` → `create`
- [ ] `PUT    /teams/:team_id/worksets/:workset_id` → `update`
- [ ] `DELETE /teams/:team_id/worksets/:workset_id` → `delete`

---

## 7. Comic (`comic` aggregate) — **new: domain model needed**

> Go refs: `app/comic.go`, `app/impl/comic.go` (612 loc), `app/val/comic.go`
> Go aggr: `domain/model/aggr/comic.go`, Go enum: `domain/model/enum/comic.go`
> ⚠️ This is one of the most complex aggregates due to `pinned_*` replica columns and OSS cover image.

### 7.1 — Domain Model

- [-] Set up `ComicAggr`, `ComicForm`, `ComicPatch`, `ComicVal` structs
  - (ref: `aggr/comic.go`)
  - Fields: `id`, `workset_id`, `name`, `desc`, `cover_key`, `cover_uploaded_at`, `chapter_count`
  - **Replica fields**: `has_pinned_chapter`, `pinned_uploaded_at`, `pinned_translating_at`, …, `pinned_published_at`
    (10 timestamp columns for pinned-chapter workflow mirroring — read-only in comic repo, written by event handlers)
  - `ComicForm` for create; `ComicPatch` for update (put semantics)
- [ ] Add `ComicVal` read-model with all display fields
- [ ] Define `ComicQueryKind` / filter enum (by workset, by id, with workflow filters)
- [ ] Define `ComicWorkflowFilter` enum for list filtering (uses `pinned_*` replica fields)

### 7.2 — Domain Query Trait

- [-] Create `src/domain/query/comic.rs` with `ComicQuery` + `ComicRepoTransactional`
  - `ComicQuery`: `get_by_id`, `list_by_workset` (paginated, workflow filter support)
  - `ComicRepoTransactional`: `create`, `update`, `delete`, `inc_chapter_count`, `dec_chapter_count`
  - `update_pinned_replicas` (used by event handlers, not repo directly)
- [ ] Register `ComicRepoForward` in harness

### 7.3 — Infra Query

- [ ] Create `src/infra/query/entity/comic.rs` — `ComicRow`
  - Soft-delete table: include `deleted_at`
- [ ] Add `comic_table` to `schema.rs`
- [ ] Create `src/infra/query/comic.rs` — Diesel impl
- [ ] Create `src/infra/query/memory_mock/comic.rs` — mock

### 7.4 — Use Cases

- [ ] `list(cx, curr_uid, args: ListArgs) → [ComicVal]`
  - (ref: `app/impl/comic.go` `List`)
  - `curr_uid` must be a member of the owning team
  - Paginated; support workflow filter via `pinned_*` replica columns
- [ ] `create(cx, curr_uid, args: CreateArgs) → CreatedRes`
  - (ref: `app/impl/comic.go` `Create`)
  - Admin-only; create comic under a workset; init `chapter_count` = 0
- [ ] `update(cx, curr_uid, args: UpdArgs) → None`
  - (ref: `app/impl/comic.go` `Update`)
  - Admin-only; put semantics for name, description, cover
- [ ] `get_by_id(cx, curr_uid, args: GetByIdArgs) → ComicVal`
  - (ref: `app/impl/comic.go` `GetById`)
  - `curr_uid` must be a member of the owning team
  - Preload workset + team info
- [ ] `resv_cover(cx, curr_uid, args: ResvCoverArgs) → ResvCoverRes`
  - (ref: `app/impl/comic.go` `ResvCover`)
  - Admin-only; reserve OSS PUT URL for comic cover
  - Generate `comic_covers/<comic_id>/<random>.{ext}` key
- [ ] `mark_cover_uploaded(cx, curr_uid, comic_id) → None`
  - (ref: `app/impl/comic.go` `MarkCoverUploaded`)
  - Admin-only; confirm cover upload
- [ ] `delete(cx, curr_uid, comic_id) → None`
  - (ref: `app/impl/comic.go` `Delete`)
  - Admin-only; soft delete
  - ⚠️ **Cascade**: delete all chapters under comic + their pages/units/assignments

### 7.5 — API Handlers

- [ ] `GET    /worksets/:workset_id/comics` → `list`
- [ ] `POST   /worksets/:workset_id/comics` → `create`
- [ ] `PUT    /worksets/:workset_id/comics/:comic_id` → `update`
- [ ] `GET    /worksets/:workset_id/comics/:comic_id` → `get_by_id`
- [ ] `POST   /comics/:comic_id/cover/reserve` → `resv_cover`
- [ ] `POST   /comics/:comic_id/cover/uploaded` → `mark_cover_uploaded`
- [ ] `DELETE /worksets/:workset_id/comics/:comic_id` → `delete`

---

## 8. Chapter (`chapter` aggregate) — **new: domain model needed**

> Go refs: `app/chapter.go`, `app/impl/chapter.go` (673 loc), `app/val/chapter.go`
> Go aggr: `domain/model/aggr/chapter.go`, Go enum: `domain/model/enum/chapter.go`, `workflow.go`
> ⚠️ Most complex aggregate: 9-phase workflow state machine + pinned-unique invariant + events.

### 8.1 — Domain Model

- [-] Set up `ChapterAggr`, `ChapterForm`, `ChapterPatch`, `ChapterVal` structs
  - (ref: `aggr/chapter.go`)
  - Fields: `id`, `comic_id`, `name`, `order`, `pinned`, `is_published`
  - **Workflow timestamps** (9 phases × 2 columns = 18 nullable timestamps):
    `raw_provided_at`, `raw_providing_at`, `cleaned_at`, `cleaning_at`,
    `proofread_at`, `proofreading_at`, `translated_at`, `translating_at`,
    `typeset_at`, `typesetting_at`,
    `translation_proofread_at`, `translation_proofreading_at`,
    `retyped_at`, `retyping_at`,
    `quality_checked_at`, `quality_checking_at`,
    `published_at`, `publishing_at`
  - Additional fields: `uploaded_at`, `uploading_at`, `page_count`
- [ ] Define `WorkflowPhase` enum: `Pending / Ongoing / Completed`
  - (ref: `domain/model/enum/workflow.go`)
  - Each workflow step has `<phase>_at` (completed) and/or `<phase>ing_at` (started) columns
  - Derive current phase state from nullable timestamps: both NULL → Pending; only `ing_at` → Ongoing; `_at` → Completed
- [ ] Define `ChapterWorkflow` helper methods on `ChapterAggr`:
  - `start_phase(phase)` → sets `{phase}ing_at = now`
  - `complete_phase(phase)` → sets `{phase}_at = now`, clears `{phase}ing_at`
  - Phase can progress independently (e.g., typeset complete while proofread ongoing)
- [ ] **Critical invariant**: One pinned chapter per comic
  - On `Create`: unpin all existing chapters under the same comic, then insert new with `pinned=true`
  - This MUST happen atomically in a DB transaction (`SELECT FOR UPDATE` or `UPDATE` + `INSERT`)

### 8.2 — Domain Query Trait

- [-] Create `src/domain/query/chapter.rs` with `ChapterQuery` + `ChapterRepoTransactional`
  - `ChapterQuery`: `get_by_id`, `list_by_comic` (paginated), `get_pinned_by_comic`
  - `ChapterRepoTransactional`: `create` (with pin-unset), `update`, `delete`
  - `inc_page_count`, `dec_page_count`, `update_workflow_timestamp`
- [ ] Register `ChapterRepoForward` in harness

### 8.3 — Infra Query

- [ ] Create `src/infra/query/entity/chapter.rs` — `ChapterRow`
  - Soft-delete table
- [ ] Add `chapter_table` to `schema.rs`
- [ ] Create `src/infra/query/chapter.rs` — Diesel impl
  - ⚠️ `create` must run in transaction: `UPDATE SET pinned=false WHERE comic_id=...` then `INSERT`
  - Use `diesel::update` with scoped filter
- [ ] Create `src/infra/query/memory_mock/chapter.rs` — mock

### 8.4 — Use Cases

- [ ] `list(cx, curr_uid, args: ListArgs) → [ChapterVal]`
  - (ref: `app/impl/chapter.go` `List`)
  - `curr_uid` must be a member of the owning team (walk: chapter→comic→workset→team)
  - Paginated
- [ ] `get_by_id(cx, curr_uid, args: GetByIdArgs) → ChapterVal`
  - (ref: `app/impl/chapter.go` `GetById`)
  - `curr_uid` must be a member of the owning team
- [ ] `get_pinned(cx, curr_uid, comic_id) → ChapterVal`
  - (ref: `app/impl/chapter.go` `GetPinned`)
  - Return the single pinned chapter for a comic
- [ ] `create(cx, curr_uid, args: CreateArgs) → CreatedRes`
  - (ref: `app/impl/chapter.go` `Create`)
  - Admin-only; create chapter with pinned=true, atomically unpin others
  - Emit pinned-chapter events that trigger comic `pinned_*` replica updates
  - Increment comic's `chapter_count`
- [ ] `update(cx, curr_uid, args: UpdArgs) → None`
  - (ref: `app/impl/chapter.go` `Update`)
  - Admin-only; put semantics for name, order, workflow timestamps
  - May emit workflow-change events
- [ ] `join(cx, curr_uid, args: JoinArgs) → AssignmentVal`
  - (ref: `app/impl/chapter.go` `Join`)
  - `curr_uid` must be a member of the owning team
  - Add current user to chapter's assignment list with specified roles
  - Returns created `AssignmentVal`
- [ ] `delete(cx, curr_uid, chapter_id) → None`
  - (ref: `app/impl/chapter.go` `Delete`)
  - Admin-only; soft delete
  - ⚠️ **Cascade**: delete all pages under chapter + their units, and all assignments

### 8.5 — API Handlers

- [ ] `GET    /comics/:comic_id/chapters` → `list`
- [ ] `POST   /comics/:comic_id/chapters` → `create`
- [ ] `GET    /comics/:comic_id/chapters/pinned` → `get_pinned`
- [ ] `GET    /comics/:comic_id/chapters/:chapter_id` → `get_by_id`
- [ ] `PUT    /comics/:comic_id/chapters/:chapter_id` → `update`
- [ ] `POST   /comics/:comic_id/chapters/:chapter_id/join` → `join` (or `/chapters/:chapter_id/join`)
- [ ] `DELETE /comics/:comic_id/chapters/:chapter_id` → `delete`

---

## 9. Page (`page` aggregate) — **new: domain model needed**

> Go refs: `app/page.go`, `app/impl/page.go` (466 loc), `app/val/page.go`
> Go aggr: `domain/model/aggr/page.go`

### 9.1 — Domain Model

- [-] Set up `PageAggr`, `PageForm`, `PagePatch`, `PageVal` structs
  - (ref: `aggr/page.go`)
  - Fields: `id`, `chapter_id`, `order`, `oss_key`, `is_uploaded`
  - Hard-delete table
  - ID generation via cuid2
- [ ] Add `ResvChapterPagesRes` value types for batch reservation

### 9.2 — Domain Query Trait

- [-] Create `src/domain/query/page.rs` with `PageQuery` + `PageRepoTransactional`
  - `PageQuery`: `list_by_chapter` (by order), `get_by_id`
  - `PageRepoTransactional`: `batch_create`, `delete_by_id`, `delete_by_chapter`, `update_upload_status`
- [ ] Register `PageRepoForward` in harness

### 9.3 — Infra Query

- [ ] Create `src/infra/query/entity/page.rs` — `PageRow` (hard-delete, no `deleted_at`)
- [ ] Add `page_table` to `schema.rs` (if new migration needed)
- [ ] Create `src/infra/query/page.rs` — Diesel impl
- [ ] Create `src/infra/query/memory_mock/page.rs` — mock

### 9.4 — Use Cases

- [ ] `resv_chapter_pages(cx, curr_uid, args: ResvChapterPagesArgs) → ResvChapterPagesRes`
  - (ref: `app/impl/page.go` `ResvChapterPages`)
  - Admin-only; batch-reserve OSS PUT URLs for N pages
  - Insert `PageRow`s with `is_uploaded=false`, generate OSS keys + signed URLs
  - Return list of (page_id, oss_key, signed_url)
- [ ] `resv_chapter_page(cx, curr_uid, args: ResvChapterPageArgs) → ResvChapterPageRes`
  - (ref: `app/impl/page.go` `ResvChapterPage`)
  - Admin-only; reserve OSS PUT URL for a single page re-upload
  - Invalidate previous reservation (update oss_key)
- [ ] `list(cx, curr_uid, args: ListArgs) → [PageVal]`
  - (ref: `app/impl/page.go` `List`)
  - `curr_uid` must be a member of the owning team (walk: page→chapter→comic→workset→team)
  - Ordered by `order ASC`
- [ ] `mark_image_uploaded(cx, curr_uid, args: MarkUploadedArgs) → None`
  - (ref: `app/impl/page.go` `MarkImageUploaded`)
  - Admin-only; set `is_uploaded = true`
- [ ] `delete_by_chapter_id(cx, curr_uid, chapter_id) → None`
  - (ref: `app/impl/page.go` `DeleteByChapterId`)
  - Admin-only; hard delete all pages under a chapter + their units
  - ⚠️ **Cascade**: also delete all units for each deleted page

### 9.5 — API Handlers

- [ ] `POST   /chapters/:chapter_id/pages/reserve` → `resv_chapter_pages`
- [ ] `POST   /chapters/:chapter_id/pages/:page_id/reserve` → `resv_chapter_page` (re-upload)
- [ ] `GET    /chapters/:chapter_id/pages` → `list`
- [ ] `POST   /chapters/:chapter_id/pages/:page_id/uploaded` → `mark_image_uploaded`
- [ ] `DELETE /chapters/:chapter_id/pages` → `delete_by_chapter_id`

---

## 10. Unit (`unit` aggregate) — **new: domain model needed**

> Go refs: `app/unit.go`, `app/impl/unit.go` (207 loc), `app/val/unit.go`
> Go aggr: `domain/model/aggr/unit.go`, `domain/model/aggr/unit_iface.go`

### 10.1 — Domain Model

- [-] Set up `UnitAggr`, `UnitForm`, `UnitPatch`, `UnitVal` structs
  - (ref: `aggr/unit.go`)
  - Fields: `id`, `page_id`, `text` (nullable), `x_coord` (REAL→f64), `y_coord` (REAL→f64),
    `width` (REAL→f64), `height` (REAL→f64)
  - Hard-delete table
- [ ] Add `PageUnitDiff` / `SavePageUnitsRes` value types for the diff-based save operation
  - (ref: `app/val/unit.go`)
  - Diff contains: `created`, `updated`, `deleted` lists of units

### 10.2 — Domain Query Trait

- [-] Create `src/domain/query/unit.rs` with `UnitQuery` + `UnitRepoTransactional`
  - `UnitQuery`: `list_by_page`
  - `UnitRepoTransactional`: `batch_upsert`, `batch_delete_by_ids`, `delete_by_page_id`
- [ ] Register `UnitRepoForward` in harness

### 10.3 — Infra Query

- [ ] Create `src/infra/query/entity/unit.rs` — `UnitRow` (hard-delete)
- [ ] Add `unit_table` to `schema.rs`
- [ ] Create `src/infra/query/unit.rs` — Diesel impl
  - `batch_upsert`: might need raw SQL or `INSERT … ON CONFLICT DO UPDATE` (Diesel upsert)
- [ ] Create `src/infra/query/memory_mock/unit.rs` — mock

### 10.4 — Use Cases

- [ ] `list_by_page(cx, curr_uid, args: ListArgs) → ListRes`
  - (ref: `app/impl/unit.go` `ListByPage`)
  - `curr_uid` must be a member of the owning team
  - Return all units for a page
- [ ] `save_by_page(cx, curr_uid, args: SaveArgs) → SaveRes`
  - (ref: `app/impl/unit.go` `SaveByPage`)
  - Admin-only; apply a diff (create/update/delete) to a page's units in one transaction
  - Sync unit counters (if Go has such logic)

### 10.5 — API Handlers

- [ ] `GET    /pages/:page_id/units` → `list_by_page`
- [ ] `PUT    /pages/:page_id/units` → `save_by_page`

---

## 11. Assignment (`assignment` aggregate) — **new: domain model needed**

> Go refs: `app/assignment.go`, `app/impl/assignment.go` (329 loc), `app/val/assignment.go`
> Go aggr: `domain/model/aggr/assignment.go`

### 11.1 — Domain Model

- [-] Set up `AssignmentAggr`, `AssignmentForm`, `AssignmentPatch`, `AssignmentVal` structs
  - (ref: `aggr/assignment.go`)
  - Fields: `id`, `user_id`, `chapter_id`, `role_mask` (u32 with 7 role flags)
  - Role timestamps: `assigned_raw_provider_at` … `assigned_admin_at` (7 nullable timestamps)
  - Hard-delete table
- [ ] Define `AssignmentRoleMask` bitflag type (reuse or extend `RoleMask` from `value/role.rs`)
  - ⚠️ Assignment roles and Member roles may differ? Check Go refs. Assignment has same 7 roles.

### 11.2 — Domain Query Trait

- [-] Create `src/domain/query/assignment.rs` with `AssignmentQuery` + `AssignmentRepoTransactional`
  - `AssignmentQuery`: `list_by_chapter` (preload user), `list_by_user` (across chapters/comics/worksets)
  - `AssignmentRepoTransactional`: `upsert`, `delete`
- [ ] Register `AssignmentRepoForward` in harness

### 11.3 — Infra Query

- [ ] Create `src/infra/query/entity/assignment.rs` — `AssignmentRow` (hard-delete)
- [ ] Add `assignment_table` to `schema.rs`
- [ ] Create `src/infra/query/assignment.rs` — Diesel impl
- [ ] Create `src/infra/query/memory_mock/assignment.rs` — mock

### 11.4 — Use Cases

- [ ] `list_by_chapter(cx, curr_uid, args: ListByChapterArgs) → [AssignmentVal]`
  - (ref: `app/impl/assignment.go` `ListByChapter`)
  - `curr_uid` must be a member of owning team
  - Preload user info (nickname, avatar)
- [ ] `list_by_user(cx, curr_uid, args: ListByUserArgs) → [AssignmentVal]`
  - (ref: `app/impl/assignment.go` `ListByUser`)
  - List all assignments across chapters for `curr_uid`
  - Optionally filter by workset/comic
  - Paginated
- [ ] `upsert(cx, curr_uid, args: UpsertArgs) → None`
  - (ref: `app/impl/assignment.go` `Upsert`)
  - Admin-only; put semantics for role mask
  - If role mask is zero → redirect to `delete`
  - Insert or update existing assignment for (user, chapter) pair
- [ ] `delete(cx, curr_uid, assignment_id) → None`
  - (ref: `app/impl/assignment.go` `Delete`)
  - Admin-only; hard delete one assignment

### 11.5 — API Handlers

- [ ] `GET    /chapters/:chapter_id/assignments` → `list_by_chapter`
- [ ] `GET    /me/assignments` → `list_by_user`
- [ ] `PUT    /chapters/:chapter_id/assignments` → `upsert` (or `/assignments/upsert`)
- [ ] `DELETE /chapters/:chapter_id/assignments/:assignment_id` → `delete`

---

## 12. Assignment Invitation (`assignment_invitation` aggregate) — **new: domain model needed**

> Go refs: `app/assignment_invitation.go`, `app/impl/assignment_invitation.go` (289 loc), `app/val/assignment_invitation.go`
> Go aggr: `domain/model/aggr/assignment_inv.go`

### 12.1 — Domain Model

- [-] Set up `AssignmentInvitationAggr`, `AssignmentInvitationForm`, `AssignmentInvitationVal` structs
  - (ref: `aggr/assignment_inv.go`)
  - Fields: `id`, `chapter_id`, `invitor_id`, `invitee_id` (nullable?), `code` (unique), `roles`, `pending`
  - Hard-delete table
- [ ] Add `CreateAssignmentInvRes` value type

### 12.2 — Domain Query Trait

- [-] Create `src/domain/query/assignment_invitation.rs` with `AssignmentInvitationQuery` + `AssignmentInvitationRepoTransactional`
  - `AssignmentInvitationQuery`: `list_by_chapter`, `get_by_code`
  - `AssignmentInvitationRepoTransactional`: `create`, `delete`, `mark_used`
- [ ] Register `AssignmentInvitationRepoForward` in harness

### 12.3 — Infra Query

- [ ] Create `src/infra/query/entity/assignment_invitation.rs` — `AssignmentInvitationRow`
- [ ] Add `invitation_table` (or `assignment_invitation_table`) to `schema.rs`
  - ⚠️ Check Go migration: table name might be `assignment_invitation_table` to avoid conflict with member `invitation_table`
- [ ] Create `src/infra/query/assignment_invitation.rs` — Diesel impl
- [ ] Create `src/infra/query/memory_mock/assignment_invitation.rs` — mock

### 12.4 — Use Cases

- [ ] `list_by_chapter(cx, curr_uid, args: ListArgs) → [AssignmentInvVal]`
  - (ref: `app/impl/assignment_invitation.go` `ListByChapter`)
  - `curr_uid` must be a member of owning team
- [ ] `create(cx, curr_uid, args: CreateArgs) → CreateRes`
  - (ref: `app/impl/assignment_invitation.go` `Create`)
  - Admin-only; create invitation for a chapter with roles + unique code
- [ ] `delete(cx, curr_uid, inv_id) → None`
  - (ref: `app/impl/assignment_invitation.go` `Delete`)
  - Admin-only; delete a pending invitation
- [ ] `join_by_inv_code(cx, curr_uid, args: JoinArgs) → None`
  - (ref: `app/impl/assignment_invitation.go` `JoinByInvCode`)
  - `curr_uid` must be a member of the owning team
  - Consume invitation code; create assignment record for the user

### 12.5 — API Handlers

- [ ] `GET    /chapters/:chapter_id/assignment-invitations` → `list_by_chapter`
- [ ] `POST   /chapters/:chapter_id/assignment-invitations` → `create`
- [ ] `DELETE /chapters/:chapter_id/assignment-invitations/:inv_id` → `delete`
- [ ] `POST   /assignment-invitations/join` → `join_by_inv_code`

---

## 13. Announcement (`announcement` aggregate) — **new: domain model needed**

> Go refs: `app/announcement.go`, `app/impl/announcement.go` (128 loc), `app/val/announcement.go`
> Go aggr: `domain/model/aggr/announcement.go`

### 13.1 — Domain Model

- [-] Set up `AnnouncementAggr`, `AnnouncementForm`, `AnnouncementVal` structs
  - (ref: `aggr/announcement.go`)
  - Fields: `id`, `team_id`, `author_id`, `title`, `body` (nullable), `created_at`
  - Hard-delete? Check Go ref. Likely hard-delete.
  - Preloaded fields: `author` (UserAggr reference)

### 13.2 — Domain Query Trait

- [-] Create `src/domain/query/announcement.rs` with `AnnouncementQuery` + `AnnouncementRepoTransactional`
  - `AnnouncementQuery`: `list_by_team` (paginated, ordered by `created_at DESC`)
  - `AnnouncementRepoTransactional`: `create`
  - (No delete in Go interface — confirm; check if there's a separate delete)
- [ ] Register `AnnouncementRepoForward` in harness

### 13.3 — Infra Query

- [ ] Create `src/infra/query/entity/announcement.rs` — `AnnouncementRow`
- [ ] Add `announcement_table` to `schema.rs`
- [ ] Create `src/infra/query/announcement.rs` — Diesel impl
- [ ] Create `src/infra/query/memory_mock/announcement.rs` — mock

### 13.4 — Use Cases

- [ ] `list(cx, curr_uid, args: ListArgs) → [AnnouncementVal]`
  - (ref: `app/impl/announcement.go` `List`)
  - `curr_uid` must be a member of the team
  - Paginated; preload author info
- [ ] `create(cx, curr_uid, args: CreateArgs) → CreatedRes`
  - (ref: `app/impl/announcement.go` `Create`)
  - Admin-only; create team announcement
  - No events emitted (check Go ref — likely just a simple insert)

### 13.5 — API Handlers

- [ ] `GET    /teams/:team_id/announcements` → `list`
- [ ] `POST   /teams/:team_id/announcements` → `create`

---

## 14. Comment (`comment` aggregate) — **new: domain model needed**

> Go refs: `app/comment.go`, `app/impl/comment.go` (128 loc), `app/val/comment.go`
> Go aggr: `domain/model/aggr/comment.go`

### 14.1 — Domain Model

- [-] Set up `CommentAggr`, `CommentForm`, `CommentVal` structs
  - (ref: `aggr/comment.go`)
  - Fields: `id`, `team_id`, `author_id`, `body`, `created_at`
  - Hard-delete? Probably. Preloaded: `author` (UserAggr reference)
  - Similar to Announcement but with different table

### 14.2 — Domain Query Trait

- [-] Create `src/domain/query/comment.rs` with `CommentQuery` + `CommentRepoTransactional`
  - `CommentQuery`: `list_by_team` (paginated, ordered by `created_at DESC`)
  - `CommentRepoTransactional`: `create`
- [ ] Register `CommentRepoForward` in harness

### 14.3 — Infra Query

- [ ] Create `src/infra/query/entity/comment.rs` — `CommentRow`
- [ ] Add `comment_table` to `schema.rs`
- [ ] Create `src/infra/query/comment.rs` — Diesel impl
- [ ] Create `src/infra/query/memory_mock/comment.rs` — mock

### 14.4 — Use Cases

- [ ] `list(cx, curr_uid, args: ListArgs) → [CommentVal]`
  - (ref: `app/impl/comment.go` `List`)
  - `curr_uid` must be a member of the team
  - Paginated; preload author info
- [ ] `create(cx, curr_uid, args: CreateArgs) → CreatedRes`
  - (ref: `app/impl/comment.go` `Create`)
  - Member can create; no admin required? Check Go ref
  - Simple insert

### 14.5 — API Handlers

- [ ] `GET    /teams/:team_id/comments` → `list`
- [ ] `POST   /teams/:team_id/comments` → `create`

---

## 15. Chapter Port (`chapter_port` aggregate) — **new: domain model needed**

> Go refs: `app/chapter_port.go`, `app/impl/chapter_port.go` (459 loc), `app/val/chapter_port.go`
> ⚠️ This aggregate may NOT have its own database table — it operates on existing chapter/page/unit data.
> Check Go ref: likely a domain service that reads chapters/pages/units and formats them.

### 15.1 — Domain Model

- [-] Set up `ChapterExportVal` (JSON-safe export format)
  - (ref: Go `app/val/chapter_port.go`)
  - Contains chapter info + pages (with units) in a structured JSON-safe format
- [ ] Set up `ImportChapterArgs` (includes format selector + payload)
- [ ] Set up `ImportChapterRes` (import result)
- [ ] No new aggregate struct — this is a read/transform use case, not a CRUD entity

### 15.2 — Domain Query Trait

- [ ] No new trait needed — reuses `ChapterQuery`, `PageQuery`, `UnitQuery`

### 15.3 — Infra Query

- [ ] No new infra needed

### 15.4 — Use Cases

- [ ] `export(cx, curr_uid, chapter_id) → ExportVal`
  - (ref: `app/impl/chapter_port.go` `Export`)
  - `curr_uid` must be a member of the owning team
  - Fetch chapter + all pages + all units; assemble JSON-safe export object
- [ ] `export_lp(cx, curr_uid, chapter_id) → String`
  - (ref: `app/impl/chapter_port.go` `ExportLp`)
  - Same as export but formatted as LabelPlus text format
- [ ] `import(cx, curr_uid, args: ImportArgs) → ImportRes`
  - (ref: `app/impl/chapter_port.go` `Import`)
  - Admin-only; import chapter data from one supported format
  - Transaction: create/update chapter, pages, units from payload

### 15.5 — API Handlers

- [ ] `GET    /chapters/:chapter_id/export` → `export`
- [ ] `GET    /chapters/:chapter_id/export/lp` → `export_lp`
- [ ] `POST   /chapters/:chapter_id/import` → `import`

---

## 16. User Stats (`user_stats` aggregate) — **disabled in Go**

> Go refs: `app/user_stats.go`, `app/impl/user_stats.go` (3 loc)
> **Status**: Intentionally disabled in Go (`UserStatsApp` placeholder only).
> Skip for now; may be implemented later if needed.

---

## 17. Cross-Cutting Concerns

These span multiple aggregates and should be done alongside or after the core refactoring.

### 17.1 — Event System

> Go refs: `domain/model/event/`, `internal/domain/event/`, `internal/infra/event/`
> Rust has `Event`, `EventEmit`, `EffectSink` foundation (✅). Needs events for remaining aggregates.

- [ ] Define events for **Chapter** (pinned changed, workflow phase started/completed, published)
- [ ] Define events for **Comic** (pinned replicas updated)
- [ ] Define events for **Assignment** (role changed)
- [ ] Define events for **Member** (role changed, joined, left)
- [ ] Create `src/domain/model/event/chapter.rs`, `comic.rs`, `assignment.rs`, etc.
- [ ] Implement event publishing in use cases (after transaction commit)
- [ ] Implement event handlers (if any side-effects) as `EffectSink` handlers or separate services

### 17.2 — Cascade Utilities

> Go ref: `app/impl/cascade_util.go` (200 loc)
> Rust may want a shared `Cascade` service or use-case helper.

- [ ] Workset delete → cascading delete of comics, chapters, pages, units, assignments
- [ ] Comic delete → cascading delete of chapters, pages, units, assignments
- [ ] Chapter delete → cascading delete of pages, units, assignments
- [ ] Page delete → cascading delete of units
- [ ] Decide: inline in use cases or extract `CascadeHelper` / trait

### 17.3 — Role / Permission Checks

> Go: domain services (`internal/domain/svc/`) for validation logic.

- [ ] `TeamAdminGuard`: check `curr_uid` has admin role in team (used by many use cases)
- [ ] `TeamMemberGuard`: check `curr_uid` is any member of team
- [ ] `ChapterAccessGuard`: walk chapter→comic→workset→team and check membership
- [ ] Consider implementing as a reusable trait or helper function

### 17.4 — Workflow State Machine (Chapter)

> Go ref: `internal/domain/model/workflow.go`, `internal/domain/model/chapter.go`

- [ ] Implement `WorkflowPhase` enum + `derive_phase(timestamps)` logic
- [ ] Implement `start_phase`, `complete_phase` on `ChapterAggr`
- [ ] Implement pinned-unique invariant in `ChapterRepoTransactional::create`

### 17.5 — Comic `pinned_*` Replica Fields

> Go ref: `app/impl/comic_util.go` (459 loc), `app/impl/comic_log.go` (179 loc)
> These are event-driven updates that mirror pinned-chapter workflow onto comic rows.

- [ ] Implement `update_pinned_replicas` on `ComicRepoTransactional`
- [ ] Implement event handler: on chapter pinned/workflow change → update comic's pinned_* fields
- [ ] Implement `applyComicWorkflowFilter` for comic list queries
- [ ] Validate filtering logic against Go's `comic_util.go`

### 17.6 — OSS Image Integration

> Rust has `OssImagePool` + `ImageGetForward`/`ImagePutForward`/`ImageDeleteForward` (✅).

- [ ] Ensure `resv_*` use cases (user avatar, team avatar, comic cover, page images) use OSS forward correctly
- [ ] Validate key naming conventions match Go: `avatars/`, `team_avatars/`, `comic_covers/`, `chapter_pages/`

### 17.7 — Auth Token Middleware

> Rust has `TokenSign`/`TokenParse` in domain, `JwtIssuer` in infra, plus middleware basics (✅).

- [ ] Complete auth middleware: extract `curr_uid` from Bearer token, inject into request extensions
- [ ] Wire middleware into all protected routes

### 17.8 — Pagination

> Go: explicit pagination in all list use cases (offset/limit).

- [ ] Define shared `Pagination` struct in `src/usecase/data_object/` or `src/domain/model/value/`
- [ ] All list use cases return paginated results with total count
- [ ] All list query traits accept pagination parameters

---

## Suggested Refactoring Order (Dependency-Driven)

Numbers show aggregate section above. Follow sequentially:

| Step | Aggregates | Rationale |
|------|-----------|-----------|
| **A** | Complete `1. User` + `2. Team` + `3. Member` + `4. MemberInvitation` + `5. SystemMail` | Finish all remaining use cases for already-modeled aggregates that have no further deps |
| **B** | `6. Workset` | Depends on Team + Member; foundation for Comic |
| **C** | `7. Comic` | Depends on Workset |
| **D** | `8. Chapter` | Depends on Comic; most complex, includes workflow + pinned invariant |
| **E** | `9. Page` | Depends on Chapter |
| **F** | `10. Unit` | Depends on Page |
| **G** | `11. Assignment` | Depends on Chapter + User |
| **H** | `12. AssignmentInvitation` | Depends on Chapter + User |
| **I** | `13. Announcement` + `14. Comment` | Depends on Team (simple leaf aggregates) |
| **J** | `15. ChapterPort` | Depends on Chapter + Page + Unit (read-transform layer) |
| **K** | `17. Cross-Cutting Concerns` | Events, cascade, workflow, replicas, pagination — interspersed throughout |

Each step: model → query trait → infra query (entity + impl + mock) → use cases → API handler → **cargo check**.

---

## Summary Counts

| Category | Go Count | Rust Done | Rust Remaining |
|----------|----------|-----------|----------------|
| **Domain aggregates** | 15 (+1 disabled) | 5 | 10 |
| **Domain query traits** | 15 | 5 | 10 |
| **Infra query (Diesel/mock)** | 15 | 5 | 10 |
| **Use cases** | ~67 | 1 | ~66 |
| **API endpoints** | ~60 | 1 | ~59 |
