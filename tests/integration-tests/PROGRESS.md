# Integration Test Progressive Handoff

This document tracks the build-out of the modular integration test suite
described by `test-plan.md`. The suite is split into 11 progressive modules
(`src/suites/it_00_*` through `src/suites/it_10_*`) that share a single
`RunCtx` and run in dependency order. Each module is self-contained: its
header doc states its **preconditions** and **postconditions**, so a handoff
agent can implement one module at a time without understanding the whole
suite.

## How to run

```text
# 1. PostgreSQL at DATABASE_URL (see repo .env). Apply migrations: just mgr-run
# 2. Build + start the Rust HTTP server: cargo build && ./target/debug/poprako-server
# 3. From the integration-tests project root:
cd tests/integration-tests && pnpm install && pnpm api
```

`pnpm typecheck` runs `tsc --noEmit` for a fast static check without a server.

## Module status

| Module | File | Status | Covers (test-plan) |
| ------ | ---- | ------ | ------------------ |
| it_00 | `it_00_bootstrap_auth_default_seed.ts` | **DONE** | A1, A2 |
| it_01 | `it_01_member_invitation_register_roles.ts` | **DONE** | B1, B2, B3, B4, B5 |
| it_02 | `it_02_workset_comic_chapter_index.ts` | **DONE** | C1, C2, C3, C4, C5, C6, C7 |
| it_03 | `it_03_page_reserve_image.ts` | **DONE** | D1, D2, D3 |
| it_04 | `it_04_assignment_invitation.ts` | **DONE** | E1, E2, E3 |
| it_05 | `it_05_unit_save_order_count.ts` | **DONE** | F1, F2, F3, F4, F5, F10 |
| it_06 | `it_06_unit_concurrency.ts` | **DONE** | F6, F7, F8, F9 |
| it_07 | `it_07_workflow_sysmail.ts` | **DONE** | G1, G2, G3, G4, G5 |
| it_08 | `it_08_info_update_upload_mark.ts` | **DONE** | C8, H1, H2, H3 |
| it_09 | `it_09_cross_team_permission.ts` | **DONE** | I1 |
| it_10 | `it_10_cascade_delete_cleanup.ts` | **DONE** | C9 |

A stub's `IMPLEMENTED` export is `false`; `main.ts` imports that flag and
registers the module with `{ skip: true }`, so the run is green and the stub
shows as a skipped subtest. The flag is the **single source of truth** — to
implement a module, flip `IMPLEMENTED = true` in the module file and replace
the `throw` body with the implementation guided by the file's header comment.
`main.ts` picks the flag up automatically; no other wiring is needed.

## Shared infrastructure (do not reinvent)

| File | Purpose |
| ---- | ------- |
| `src/state/roles.ts` | Role bitmask constants matching `src/value/role.rs`. |
| `src/state/stages.ts` | Stage names, phase decoder, advance-count table. |
| `src/state/prefix.ts` | Per-run unique prefix; `qid/nickname/password/titled`. |
| `src/state/runCtx.ts` | `RunCtx` + `UserClient` + persona matrix. |
| `src/http/apiClient.ts` | `ApiClient` + `clientFor` + raw-text response. |
| `src/http/assertions.ts` | `expectStatus/NoContent/SuccessData/SuccessList/Error/RawBody/StatusIn/NoServerError`. |
| `src/http/types.ts` | All `*Val` interfaces mirroring `src/data/*.rs`. |
| `src/http/fixtures.ts` | Reusable building blocks (login, register, create\*, list\*, unit oper builders, export/import). |
| `src/http/invariants.ts` | J1–J6 invariant assertions + `assertSubtreeInvariants` + `assertStagesPipelineConsistent`. |
| `src/db/seed.ts` | `resetDatabase`, `assertDatabaseIsSeedOnly`, `cleanupToSeed`, `grantChapterWorkerRoles`. |

## Grounded behaviour pins (verified against Rust source)

The test-plan.md expected codes were adjusted to match the real server. Any
handoff agent MUST keep these pins; if a real run disproves one, update this
table and the affected module in the same change.

| Plan expectation | Real behaviour | Source |
| --- | --- | --- |
| Duplicate pending invitation → 409 | **422 code 2** `error-already-exists` | partial unique index `uidx_member_invitation_team_id_invitee_qid_pending` + `rdb_core::diesel` maps UniqueViolation → `Expected::Args` |
| Register with deleted / consumed / wrong-qid code → 401 | **422 code 2** `error-invalid-invitation-code` / not-found | `auth::register` uses `get_info_by_code_excluded` (excludes consumed) and qid compare → `Expected::Args` |
| Composite `role` query filter → 422 code 2 | **422 raw serde rejection** (no `code` field) | `role: Option<RoleField>` rejects multi-bit at the query extractor; assert status only |
| `NotFound` leak → 500 | **422 code 2** `error-not-found` | `rdb_core::diesel` maps NotFound → `Expected::Args` |
| Role values REVIEWER=32, PUBLISHER=64, ADMIN=128 | confirmed | `src/value/role.rs` RoleField consts |
| Every stage needs 2 advances | **Mixed**: raw-provide/review/publish = 1 advance (one-shot); translate/proofread/typeset-redraw = 2 advances | `src/value/chapter.rs` `is_valid_stage_phase` + `try_modify_stage` |
| publish can revert | **publish CANNOT revert** (422) | `try_modify_stage` Publish+Revert branch |
| `oper` values | `advance` / `revert` (kebab-case) | `StageOper` serde rename_all |
| Unit `oper` tag values | `save` / `delete` (snake_case) | `UnitOperData` serde tag="oper" |
| Path/body id mismatch | **422 code 7** | `HttpError::unprocessable` |
| Team/user/member/assignment unique-index conflicts | **422 code 2** `error-already-exists` | `rdb_core::diesel` |
| workset/comic/chapter create/update/delete | team **ADMIN** only (sadmin is the only admin in seed) | `complex/workset|comic|chapter.rs` `check_user_is_team_admin` |
| chapter create auto-creates an ADMIN assignment for the creator | confirmed | `usecase/comic.rs` + `usecase/chapter.rs` |
| chapter pin/subtitle patch perm | **chapter admin assignment** (not team admin) | `complex/chapter.rs` `check_admin` |
| page reserve (batch + single) perm | assignment with **RAW_PROVIDER or REVIEWER** | `complex/page.rs` `check_reserve_role` |
| page image mark-uploaded perm | assignment with **RAW_PROVIDER** | `complex/page.rs` `check_upload_role` |
| page delete-all perm | team **ADMIN** | `complex/page.rs` `can_user_delete` |
| unit save perm | assignment with **TRANSLATOR or PROOFREADER** (RAW_PROVIDER alone CANNOT save units) | `complex/unit.rs` `can_user_save_infos` |
| assignment join duplicate | **upsert (merge_roles)**, NOT an error; `(chapter_id,user_id)` stays unique | `usecase/assignment.rs` |
| assignment join role check | member roles must **contain** the requested assignment roles; ADMIN bit -> 422/2 | `complex/assignment.rs` `check_target_roles` |
| move-via-save (Save with `id` + `before_id`) | **SUPPORTED** — removes the unit and re-inserts before `before_id` | `complex/unit.rs` `apply_opers_to_order` |
| `last_translator_id` / `last_proofreader_id` | **client-snapshot** (usecase does NOT inject token.user_id) | `usecase/unit.rs` |
| poprako export shape ≠ import shape | export `ChapterTranslationExportVal`; import `PoprakoProjectImport` (`image_filename`/`x`/`y`/`index_in_page`/`is_inbox`/`prooved_text`/`is_prooved`). `buildPoprakoImportContent` converts. | `data/chapter_port.rs` + `model/chapter_port.rs` |
| translation import perm | requires a **translator/proofreader assignment** on the chapter; target chapter must have **same page count** as import | `usecase/chapter_port/import.rs` |
| workflow stage advance perm | per-stage role (raw->RAW_PROVIDER, translate->TRANSLATOR, ...); **REVIEWERs bypass**; sadmin (ADMIN-only assignment) CANNOT advance worker stages | `complex/chapter.rs` `check_workflow_role` |
| system mail generation | **background tokio task** (AsyncEffectDevelop); mails NOT visible immediately — use `waitForMails` to poll | `part_impl/effect_async.rs` |
| `ChapterWorkflowCompleted` recipients | next-stage role (raw->TRANSLATOR, translate->PROOFREADER, proofread->TYPESETTER, typeset->REVIEWER, review->PUBLISHER) **PLUS reviewers** (progress copy) | `part_impl/effect_async/chapter.rs` `next_phase_config` + `notify_reviewers_on_progress` |
| `ChapterPublished` recipients | **REVIEWERs** (publish label) | `part_impl/effect_async/chapter.rs` |
| `ChapterWorkflowReverted` | **NO mail** (dispatches to `{}`) — plan's G4 "revert -> rework mail" is WRONG | `part_impl/effect_async/dispatch.rs` |
| user profile update perm | **SELF only** (token.user_id == data.id); sadmin CANNOT edit another user | `usecase/user.rs` `update_info` |
| user delete perm | **SELF only** (token.user_id == id); sadmin CANNOT delete another user. Cascade-deletes memberships. | `usecase/user.rs` `delete` |
| team create perm | **sadmin only**; sadmin auto-becomes ADMIN of the new team | `usecase/team.rs` `create` |
| team delete perm | team admin (sadmin for teams sadmin created); cascade-deletes worksets/comics/chapters/pages/units/members | `usecase/team.rs` `delete` |

## Stage phase cheat sheet

`stages` is a u32 with 2 bits per stage (low bits first):

| Stage | Bits | Phases | Advances to complete |
| --- | --- | --- | --- |
| raw-provide | 0–1 | Pending(0) → Completed(2) | 1 |
| translate | 2–3 | Pending → Active(1) → Completed | 2 |
| proofread | 4–5 | Pending → Active → Completed | 2 |
| typeset-redraw | 6–7 | Pending → Active → Completed | 2 |
| review | 8–9 | Pending → Completed | 1 |
| publish | 10–11 | Pending → Completed (no revert) | 1 |

Decode with `stagePhase(stagesMask, stage)` from `state/stages.ts`.

## Handoff protocol for the next module

All 11 modules are now implemented. The protocol below remains for future
module additions or re-implementation passes:

1. Pick the lowest-numbered module you want to (re-)work.
2. Read its header doc: **Preconditions**, **Postconditions**, **Covers**, and
   the step-by-step **Implementation guide**.
3. Read the grounded pins above and the shared infrastructure list; reuse
   fixtures/invariants, do not hand-roll HTTP calls.
4. Implement the body, flipping `IMPLEMENTED = true` (it already is for all
   modules).
5. Run `pnpm typecheck` then `pnpm api` (server must be running at
   `http://127.0.0.1:8888`, DB migrated with `just mgr-run`). All earlier
   modules must still pass; the (re-)worked module must pass; later modules
   must still pass.
6. If a pin is disproved by a real run, update the pin table here and the
   affected module in the same change.
7. Update `TESTCASES.md` for any case you add/remove/rename.
8. Mark the module DONE in the status table (all are DONE now).

## Cleanup model

`cleanupToSeed()` runs in the `finally` block BEFORE
`assertDatabaseIsSeedOnly()`. It deletes every non-seed row in FK-safe leaf-
first order (all schema FKs are `ON DELETE RESTRICT`). This is robust to
partial runs: if only it_00 + it_01 executed, cleanup still returns the DB
to seed-only and the assert passes. The cascade-delete *endpoint* itself is
exercised by it_10 against a dedicated subtree; `cleanupToSeed` is the safety
net that gets the whole DB back regardless of what ran.
