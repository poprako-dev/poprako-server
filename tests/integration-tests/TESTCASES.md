# Integration Test Cases

> **Maintenance rule:** every change to the integration tests under
> `tests/integration-tests/src/` MUST be reflected in this document in the same
> change. Add/remove/rename a case here whenever you touch the corresponding
> `*.ts`. Keep case IDs stable; do not renumber unless a case is removed.

This document enumerates every HTTP API integration test case driven by the
TypeScript suite at `tests/integration-tests/src/`. Cases are grouped by the
11 progressive modules in `src/suites/it_*.ts`, in the order `src/main.ts`
executes them. Module build-out status is tracked in `PROGRESS.md`.

## How to run

```text
# 1. PostgreSQL reachable at DATABASE_URL (see repo .env). Apply migrations: just mgr-run
# 2. Build and start the Rust HTTP server:
cargo build && ./target/debug/poprako-server      # listens on 127.0.0.1:8888
# 3. Run the suite from the integration-tests project root:
cd tests/integration-tests && pnpm install && pnpm api
# 4. Fast static check (no server needed):
cd tests/integration-tests && pnpm typecheck
```

The suite calls `resetDatabase()` at the start, runs the 11 modules in order,
then runs `cleanupToSeed()` + `assertDatabaseIsSeedOnly()` in the `finally`
block. Unimplemented modules are registered with `{ skip: true }` so the run
stays green during the progressive handoff (see `PROGRESS.md`).

## Response conventions

- Success body (valued): `{ "code": 0, "data": <T> }` — `HttpBody<T>`.
- Success no-content: HTTP `204` with empty body — `NoContent`.
- Error body: `{ "code": <n>, "message": "..." }` — `HttpError`.
- Raw (unenveloped) body: translation export/download endpoints return raw
  JSON or plain text without the `HttpBody` envelope.

Error code mapping (from `src/api/http/result.rs`):

| code | HTTP status | Variant                    | Meaning                                            |
| ---- | ----------- | -------------------------- | -------------------------------------------------- |
| 1    | 500         | Unrecoverable              | Infra failure only. MUST NOT be reachable by any client request. |
| 2    | 422         | `Expected::Args`           | Invalid arguments / query / not-found by id / unique-violation (`error-already-exists`) |
| 3    | 401         | `Expected::Auth`           | Unauthenticated                                    |
| 4    | 403         | `Expected::Perm`           | Forbidden / permission denied                      |
| 7    | 422         | `HttpError::unprocessable` | Path id does not match body id                     |

### 5xx invariant

**No client request may produce a 5xx response.** Code 1 (500) exists solely
for genuine infrastructure failures and is concealed from clients. A leaked
`DieselError::NotFound` is mapped to `Expected::Args` (422, code 2). A
unique-violation is mapped to `Expected::Args` (422, code 2,
`error-already-exists`). Query-param deserialization failures (e.g. composite
`role` filter, non-enum `stage`) produce a 422 raw serde rejection with no
`code` field — these cases assert the status only. Every case below asserts
an exact status + code (or exact status for raw serde rejections), so a
regression that turns any 4xx into a 5xx fails the suite.

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
`resetDatabase()` (via `cleanupToSeed()`), so it verifies the suite self-
cleans. `cleanupToSeed()` deletes every non-seed row in FK-safe leaf-first
order (all schema FKs are `ON DELETE RESTRICT`), so partial runs (where only
some modules are implemented) still pass the seed-only assert.

## Shared context

Carried across modules via `RunCtx` (`src/state/runCtx.ts`):

- `sadmin`: authenticated `ApiClient` for the seed super-admin.
- `users`: `Map<persona, UserClient>` (one client per registered persona).
- `ids`: `defaultTeamId/defaultUserId/defaultMemberId` + `worksetIds` /
  `comicIds` / `firstChapterIds` maps keyed by label.
- `personas`: the 14-persona member matrix.
- `main`: `ChapterRefs` for the high-traffic `星尘旅人 / 第 2 话 月面信号`
  chapter (page ids, assignment ids).
- `auxChapters`: `Map<label, ChapterRefs>` for destructive tests (cascade,
  D3, F5, F10).
- `secondTeam`: `{ teamId, outsider }` for cross-team isolation.
- `leftoverCommentIds` / `leftoverAnnouncementIds`: recorded for explicit
  cleanup traceability (cleanup already deletes all such rows).

## Role / stage constants

`src/state/roles.ts` and `src/state/stages.ts` mirror the Rust enums exactly:

- Roles: RAW_PROVIDER=1, TRANSLATOR=2, PROOFREADER=4, TYPESETTER=8,
  REDRAWER=16, REVIEWER=32, PUBLISHER=64, ADMIN=128, BOT=256.
- Stages (kebab-case): raw-provide, translate, proofread, typeset-redraw,
  review, publish. One-shot stages (raw-provide, review, publish) take 1
  advance; three-phase stages (translate, proofread, typeset-redraw) take 2.
  publish cannot revert.

---

## Module it_00 — `suites/it_00_bootstrap_auth_default_seed.ts` (DONE)

Covers test-plan A1 (sadmin login + default-data discovery) and A2
(unauthenticated-access protection).

| ID    | Method | Path                                     | Body                              | Expected      | Notes                                                              |
| ----- | ------ | ---------------------------------------- | --------------------------------- | ------------- | ------------------------------------------------------------------ |
| A1-01 | POST   | `/api/v1/auth/login`                     | `{ qid:"123456", password:"123456" }` | 200, code 0; `data.user_id` == seed user; `token` > 20 chars; sadmin client token set | |
| A1-02 | GET    | `/api/v1/users/me`                       | — (bearer)                        | 200, code 0; `id` == seed user; `is_sadmin` == true                |                                                                    |
| A1-03 | GET    | `/api/v1/members/me?offset=0&limit=20`   | —                                 | 200, code 0; ≥1 member; default-team member has `team` embedded    |                                                                    |
| A1-04 | GET    | `/api/v1/teams?offset=0&limit=50`        | —                                 | 200, code 0; includes default team                                |                                                                    |
| A1-05 | GET    | `/api/v1/teams/{defaultTeamId}`          | —                                 | 200, code 0; `workset_next_index` non-negative int; timestamps Unix-ms int; `created_at <= updated_at` | |
| A2-01 | GET    | `/api/v1/users/me`                       | — (no token)                      | 401, code 3    | anon client                                                        |
| A2-02 | GET    | `/api/v1/members/me?offset=0&limit=20`   | — (no token)                      | 401, code 3    |                                                                    |
| A2-03 | GET    | `/api/v1/teams?offset=0&limit=20`        | — (no token)                      | 401, code 3    |                                                                    |
| A2-04 | POST   | `/api/v1/worksets`                       | `{ name, description, team_id }` (no token) | 401, code 3 | |
| A2-05 | POST   | `/api/v1/auth/logout`                    | — (throwaway client)              | 204            | clears token; subsequent `/users/me` → 401 code 3                  |
| A2-06 | GET    | `/api/v1/users/me`                       | — (after logout)                  | 401, code 3    |                                                                    |

## Module it_01 — `suites/it_01_member_invitation_register_roles.ts` (DONE)

Covers test-plan B1 (batch invite 14), B2 (modify/delete invitation), B3 (14
register + close the loop), B4 (member list filters + bad params), B5 (member
role update + permission boundary). Uses a per-run prefix for all qids/
nicknames so repeated runs do not collide.

| ID    | Method | Path                                                                       | Body / Query                                                  | Expected      | Notes                                                              |
| ----- | ------ | -------------------------------------------------------------------------- | ------------------------------------------------------------- | ------------- | ------------------------------------------------------------------ |
| B1-01 | POST   | `/api/v1/member-invitations`                                              | `{ invitee_qid, roles, team_id }` x14                         | 201, code 0 each; 14 unique `code` values; ids stored into RunCtx  | sadmin invites 14 personas |
| B1-02 | GET    | `/api/v1/teams/{teamId}/member-invitations?pending=true&offset=0&limit=100` | —                                                           | 200, code 0; includes all 14; each `pending=true`, `team_id`, `invitee_qid`, `roles`, `invitor_id` == sadmin | |
| B1-03 | POST   | `/api/v1/member-invitations`                                              | duplicate `(team_id, invitee_qid)` still pending              | 422, code 2   | partial unique index → `error-already-exists` (plan's 409 adjusted) |
| B2-01 | PUT    | `/api/v1/member-invitations/{id}/roles`                                   | `{ id, roles: RAW|TRANSLATOR }` (guest_01)                    | 204            | widens guest_01 roles                                              |
| B2-02 | GET    | `/api/v1/teams/{teamId}/member-invitations?pending=true&...`              | —                                                             | 200; guest_01 `roles` == RAW|TRANSLATOR                            |                                                                    |
| B2-03 | PUT    | `/api/v1/member-invitations/{id}/roles`                                   | `{ id: "not-the-path-id", roles }`                            | 422, code 7    | path/body id mismatch                                              |
| B2-04 | POST   | `/api/v1/member-invitations`                                              | throwaway `cancelled_01`                                      | 201            | then DELETE below                                                  |
| B2-05 | DELETE | `/api/v1/member-invitations/{id}`                                         | —                                                             | 204            | deletes the throwaway                                              |
| B2-06 | POST   | `/api/v1/auth/register`                                                   | `{ code: deletedCode, ... }`                                  | 422, code 2    | deleted code invalid (plan's 401 adjusted)                         |
| B2-07 | DELETE | `/api/v1/member-invitations/{id}`                                         | — (second time)                                               | 422, code 2    | already deleted → not-found                                        |
| B3-01 | POST   | `/api/v1/auth/register`                                                   | `{ qid, nickname, password, code }` x14                       | 201, code 0 each; `user_id` + `token`; fresh client stored into RunCtx | |
| B3-02 | GET    | `/api/v1/users/me`                                                         | — (per new user)                                              | 200; `id`/`qid`/`nickname` correct; `is_sadmin` == false            |                                                                    |
| B3-03 | GET    | `/api/v1/members/me?offset=0&limit=50`                                     | — (per new user)                                              | 200; exactly 1 member; `team_id` == default; `roles` == invitation roles | |
| B3-04 | POST   | `/api/v1/auth/register`                                                   | reuse an already-consumed `code`                              | 422, code 2    | consumed code excluded from lookup                                 |
| B3-05 | POST   | `/api/v1/auth/register`                                                   | trans_01 `code` + trans_02 `qid`                              | 422, code 2    | qid mismatch → `error-invalid-invitation-code`                     |
| B3-06 | GET    | `/api/v1/teams/{teamId}/member-invitations?pending=true&...`              | —                                                             | 200; none of the 14 still pending                                  |                                                                    |
| B3-07 | GET    | `/api/v1/teams/{teamId}/member-invitations?pending=false&...`             | —                                                             | 200; all 14 present with `pending=false`                           |                                                                    |
| B4-01 | GET    | `/api/v1/members?team_id={teamId}&incl=user&offset=0&limit=50`            | —                                                             | 200; 15 members (sadmin + 14); `incl=user` embeds matching user    |                                                                    |
| B4-02 | GET    | `/api/v1/members?team_id={teamId}&role={TRANSLATOR}&offset=0&limit=50`    | —                                                             | 200; only members with translator bit (incl. widened guest_01)     |                                                                    |
| B4-03 | GET    | `/api/v1/members?team_id={teamId}&fuzzy_nickname={prefix}trans&...`       | —                                                             | 200; only nicknames containing `prefix+trans`                      |                                                                    |
| B4-04 | GET    | `/api/v1/members/me?offset=0&limit=50`                                     | — (trans_01 client)                                           | 200; trans_01's default-team membership                            |                                                                    |
| B4-05 | GET    | `/api/v1/members?team_id={teamId}&owner_id={trans01Id}&...`               | —                                                             | 422, code 2    | both team_id and owner_id → `error-team-or-user-required`          |
| B4-06 | GET    | `/api/v1/members?owner_id={trans01Id}&role={TRANSLATOR}&...`              | —                                                             | 422, code 2    | owner mode + role                                                  |
| B4-07 | GET    | `/api/v1/members?team_id={teamId}&role={TRANSLATOR|PROOFREADER}&...`      | —                                                             | 422 (status only) | composite role: raw serde rejection, no `code` field               |
| B5-01 | PUT    | `/api/v1/members/{memberId}/roles`                                        | `{ id, roles: RAW|TRANSLATOR|PROOFREADER }` (guest_01, sadmin) | 204            | widens guest_01 member roles                                       |
| B5-02 | GET    | `/api/v1/members?team_id={teamId}&...`                                    | —                                                             | 200; guest_01 `roles` == RAW|TRANSLATOR|PROOFREADER                |                                                                    |
| B5-03 | PUT    | `/api/v1/members/{memberId}/roles`                                        | (trans_01 → proof_01)                                         | 403, code 4    | non-admin modifying another member                                 |
| B5-04 | PUT    | `/api/v1/members/{memberId}/roles`                                        | `{ id: "not-the-path-id", roles }`                            | 422, code 7    | path/body id mismatch                                              |
| B5-05 | DELETE | `/api/v1/members/{memberId}`                                              | (guest_01 → trans_01)                                         | 403, code 4    | non-admin deleting another member                                  |
| B5-06 | GET    | `/api/v1/members?team_id={teamId}&...`                                    | —                                                             | 200; still 15 members (core 14 not deleted)                        |                                                                    |

## Modules it_02 – it_10 (DONE)

All 11 modules are now implemented (`IMPLEMENTED = true` in each file). The
case IDs each module asserts are the test-plan section IDs listed below; the
exact per-case status/code expectations live in the module header docs and
the grounded pins table in `PROGRESS.md`. When a pin disproves a plan
expectation, the module asserts the real server behaviour and the pin table
records the adjustment.

| Module | Test-plan cases | Status |
| ------ | --------------- | ------ |
| it_02 workset/comic/chapter index | C1, C2, C3, C4, C5, C6, C7 | DONE |
| it_03 page reserve/image | D1, D2, D3 | DONE |
| it_04 assignment invitation | E1, E2, E3 | DONE |
| it_05 unit save order/count | F1, F2, F3, F4, F5, F10 | DONE |
| it_06 unit concurrency | F6, F7, F8, F9 | DONE |
| it_07 workflow + sysmail | G1, G2, G3, G4, G5 | DONE |
| it_08 info update / avatar / cover / announcements / comments / profile | C8, H1, H2, H3 | DONE |
| it_09 cross-team permission | I1 | DONE |
| it_10 cascade delete | C9 | DONE |

Notable plan-vs-reality adjustments (full table in `PROGRESS.md`):

- **F1 inserter**: plan said `raw_01`; raw_01 (RAW_PROVIDER only) CANNOT save
  units (unit save requires TRANSLATOR/PROOFREADER). it_05 uses trans_01.
- **F8 inserters**: plan said raw_01/raw_02; adjusted to trans_01/trans_02
  (same reason).
- **G3/G4/G5 mail matrix**: `ChapterWorkflowReverted` produces NO mail
  (plan expected rework mail). Reviewers receive a progress copy on every
  completed stage in addition to the next-stage role mail. Mails are
  generated by a background task, so it_07 polls via `waitForMails`.
- **G2/G3/G5 stage advancer**: sadmin (ADMIN-only assignment) CANNOT advance
  worker stages. it_07 uses the actual worker assignees (raw_01 for
  raw-provide, trans_01 for translate, proof_01 for proofread, type_01 for
  typeset-redraw, review_01 for review, publish_01 for publish).
- **H3.5 user delete**: plan said sadmin deletes a throwaway user; only
  **self** can delete. it_08 has the throwaway user self-delete.
- **C8/C7 profile update perms**: team/workset/comic profile update requires
  team ADMIN (sadmin only). chapter pin/subtitle requires a chapter ADMIN
  assignment (sadmin has it from create).

## Global invariant helpers (J1–J6)

`src/http/invariants.ts` exposes reusable invariant assertions that any
module can call after a mutation. They never mutate state.

| Helper | Checks (test-plan section) |
| --- | --- |
| `assertTeamInvariant` | J1: workset_next_index >= max(index)+1; active ids/indexes unique |
| `assertWorksetInvariant` | J2: comic_count == active.length; comic_next_index monotonic; ids/indexes unique |
| `assertComicInvariant` | J3: chapter_count; chapter_next_index; ≤1 pinned; pinned endpoint consistent |
| `assertChapterInvariant` | J4: page_count; unit counts == sum(pages); page indexes contiguous 0..n-1; assignment (chapter,user) unique; stages 12-bit |
| `assertPageUnitInvariant` | J5: unit ids unique; translated count == non-empty translated_text; proofread count == is_proofread flag |
| `assertPageExportInvariant` | J5: export unit ids == list unit ids; unit_index contiguous 0..n-1 |
| `assertStagesPipelineConsistent` | J4/G: stage may advance only if prior stage Completed; one-shot stages never Active |
| `assertMailInvariant` / `assertMailReadFilterInvariant` | J6: mail ids unique; read filter consistent with `read` flag |
| `assertSubtreeInvariants` | J2+J3+J4 over a whole workset tree |
| `assertMemberListWellFormed` | J1: member (team,user) unique; required fields present |
| `assertChapterPageCountersConsistent` | J4: chapter counts == sum over pages |

---

## Bug fixes / pins applied during this revision

These adjustments make the suite match the real server behaviour (the
test-plan.md expectations were aspirational in a few spots). Details in
`PROGRESS.md` "Grounded behaviour pins".

1. **Duplicate pending invitation** — plan expected 409; actual is 422 code 2
   (`error-already-exists`) via the partial unique index
   `uidx_member_invitation_team_id_invitee_qid_pending` +
   `rdb_core::diesel` UniqueViolation mapping.
2. **Register with deleted / consumed / wrong-qid code** — plan expected 401;
   actual is 422 code 2. `auth::register` uses
   `get_info_by_code_excluded` (excludes consumed) and a qid equality check,
   both returning `Expected::Args`.
3. **Composite `role` query filter** — asserted as 422 status-only (raw serde
   rejection at the query extractor, no `code` field), not 422 code 2.
4. **Role values** — plan had REVIEWER=16/PUBLISHER=32/ADMIN=64; actual is
   REVIEWER=32/PUBLISHER=64/ADMIN=128 with a separate REDRAWER=16 and BOT=256.
5. **Stage advance counts** — plan assumed 2 advances for every stage;
   actual is 1 for raw-provide/review/publish (one-shot) and 2 for
   translate/proofread/typeset-redraw. publish cannot revert.
6. **Robust partial-run cleanup** — added `cleanupToSeed()` that deletes all
   non-seed rows in FK-safe leaf-first order, so `assertDatabaseIsSeedOnly`
   passes even when only a subset of modules is implemented.
