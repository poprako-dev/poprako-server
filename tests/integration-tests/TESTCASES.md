# HTTP Integration Test Inventory

The `it_09` suite also verifies that self/profile reads preserve activity, online heartbeats synchronously refresh user
and membership activity, and unauthorized heartbeats leave activity unchanged.

This is the maintenance inventory for the TypeScript API suite. Whenever a file under `src/suites/` is added, removed,
renamed, or materially changes scope, update this document in the same change. The executable suite and its assertions
are the source of truth; do not keep separate implementation plans or status trackers here.

## Running the suite

```text
cd tests/integration-tests
deno task check
```

For an isolated end-to-end run, configure `INTEGRATION_DATABASE_URL` and run:

```text
scripts/api-integration-test.sh
```

The script creates and drops the dedicated integration database. Do not point it at a database that contains data you
need to preserve.

When running against an API server that is already available, use:

```text
deno task api
```

If the required variables only exist in the project `.env`, use:

```text
deno task --env-file=../../.env api
```

## Suite order

`src/main.ts` resets the database, runs these modules in order, then restores the seed-only state in its `finally`
block. Every current module exports `IMPLEMENTED = true`.

| Module | File                                        | Coverage                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| ------ | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| it_00  | `it_00_bootstrap_auth_default_seed.ts`      | Seed data, login, and unauthenticated access.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| it_01  | `it_01_member_invitation_register_roles.ts` | Invitations, registration, member lists, and role perms.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| it_02  | `it_02_workset_comic_chapter_index.ts`      | Workset, comic, chapter indexes, dedicated pinning, profile updates, comic detail include expansion and invalid-query rejection, and positionally aligned comic/pinned-chapter list payloads.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| it_03  | `it_03_page_reserve_image.ts`               | Authoritative hash-plus-extension manifests, optional `new_byte_len` retention, optional original filename replacement inputs for chapter and single-page allocation, required upload lengths, duplicate-ID and count validation, object uploads, optimistic exact-generation mark semantics with immediate origin/optimized/thumbnail URLs, comic-cover fallback to the pinned chapter's first uploaded page across direct/list/nested responses, replacement, deletion, and page rebuilds.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| it_04  | `it_04_assignment_invitation.ts`            | Assignment joins, invitations, role updates, self role removal, deletion, and comic-cover fallback in deeply nested owner-assignment responses.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| it_05  | `it_05_unit_save_order_count.ts`            | Unit saves require save_id and return 200 with a created_unit_ids struct array; shared is_flagged create/default/set/clear/null behavior, tombstone restoration, unchanged workflow, page flagged statistics, PopRaKo flag round trips and legacy defaults; create/next ordering and counters; chapter-scoped non-empty single-character text search with literal metacharacters, case and selected-field semantics, NUL handling, hidden-node ordering, and the 100-match limit; Page Unit diff statistics as an ordered array of original Page IDs/indexes and independent translated/editted/revision-only counts, filtered to Pages with revision differences; removal of the editted-diffs route; non-cascading literal transform; atomic combined translation exports; opt-in original LP filenames and structured raw-ident mappings, per-page fallback, default compatibility, and download parity; non-assigned team-member exports leave typeset/redraw pending; required import modes, permission-aware PopRaKo overwrite round trips with changed-page counts, and keep-mode preservation. |
| it_06  | `it_06_unit_concurrency.ts`                 | Serializable same-Page writes with client-side 409/code 8 retries, same-anchor inserts, tombstone delete/Patch commit order, and linked-list completeness.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| it_07  | `it_07_workflow_sysmail.ts`                 | Snake_case-body workflow transitions, strongly typed immutable activity events without repository storage JSON or rendered text, pagination, and deferred system mail.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| it_08  | `it_08_info_update_upload_mark.ts`          | Resource updates, runtime avatar-size validation, avatar/cover PUTs before optimistic exact-generation mark requests, immediate origin/thumbnail URLs, idempotent repeated allocations for identical available content, stale upload rejection, announcement create/update/delete, comments, and profiles.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| it_09  | `it_09_cross_team_perm.ts`                  | Cross-team authorization isolation (including comic detail expansions and chapter activity records) and team-scoped online-user leases.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| it_10  | `it_10_cascade_delete_cleanup.ts`           | Cascade deletion, serializable member-admin retention, last-admin member/user deletion rejection, and cleanup side effects.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| it_11  | `it_11_comic_archive.ts`                    | Permanent comic archive snapshots, lifecycle list filtering, and image-delete prom records.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| it_12  | `it_12_termbase_term.ts`                    | Termbase/term lifecycle, native JSON import/export and force merge, inherited lookup, fuzzy isolation, the 200-term capacity boundary, write perms, response contracts, and termbase/comic/team cascades.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |

## Terminology cascade visibility

- it_12 team cascade: after team deletion, its termbase and term must be inaccessible. Before background cleanup,
  membership checks return 403/code 4; after physical cleanup, missing records return 422/code 2. Both exact pairs are
  accepted without assuming scheduler timing; successful reads and all other status/code pairs fail the test.
- Comic-scoped terminology cascade checks continue to require 422/code 2.

## Chapter administration and assignment role removal

- E1.4c: an assignee can leave one worker role while preserving the assignment and remaining roles.
- E1.4d: a team admin can join review, then leave review while preserving the same assignment and original worker roles;
  chapter roles never contain ADMIN.
- E1.4e: an ordinary assignee cannot add ADMIN through a self-update (422/code 2).
- E1.4g: assignment listing rejects ADMIN filters (422/code 2).
- G1.admin: a new chapter without presets has no assignments; team admin can advance and revert without a worker role
  holder.
- F10.roles: team-admin and translator exports leave typeset/redraw pending; typesetter exports start it idempotently.
- F6–F9 final state: concurrent translation edits activate translation, while exports without typesetter/redrawer
  assignments leave typeset/redraw pending.
- Worker bootstrap explicitly creates assignments; missing chapter metadata updates return 422/code 2.
- E1.4f: a team admin cannot grant ADMIN to another assignee through a role update (422/code 2).

## Shared fixtures and invariants

- `src/db/seed.ts` owns reset, cleanup (including object metadata and durable object tasks), and seed-only assertions.
- `src/http/fixtures.ts` owns reusable API operations, deterministic SHA-256 image requests, and direct
  page/avatar/cover PUT uploads using every signed response header before mark-uploaded confirmation.
- `src/http/invariants.ts` owns counter, index, workflow, export, and mail consistency assertions.
- `src/state/runCtx.ts` is the shared state passed between modules.

When a test reveals a contract change, update the tested source, the relevant module assertion, and any affected active
API document together.

## it_13 — Chapter artwork port

- AW1: assignment permission and size/extension rejection without allocation.
- AW2: signed direct PUT size/headers, optimistic confirmation, origin URL export by team members, durable export
  records, deduplication, and one completion record.
- AW3: an old confirmation after replacement allocation cannot make the new generation available; stale versions are
  rejected and completion stays idempotent.
- AW4: publication clears artwork and freezes new allocation/confirmation.
- AW5: comic archival and ancestor deletion retire artwork and record durable deletion tasks.

The main runner now includes it_13. Final database cleanup also deletes `t_chapter_artwork` rows.

## it_14 — 整章监稿导入与只读查询

一个 Chapter 逻辑上最多一份当前监稿；Issue 归属独立成稿页，成稿页序独立于翻校图源。

| ID  | 场景             | 断言                                                                                          |
| --- | ---------------- | --------------------------------------------------------------------------------------------- |
| IS1 | 权限与成稿初始化 | ADMIN 可维护成稿页；相同文件名/哈希仍创建不同 ID；普通成员可读但不能写；监稿导入要求 REVIEWER |
| IS2 | 整章导入         | 显式成稿页目标、自定义类型、中文 layer_name 原文保留、图层名称与矩形的四种组合、多行及空备注  |
| IS3 | 输入拒绝         | 空白类型/图层名称、越界/非正矩形、重复或未知成稿目标返回 422，原监稿完整保留                  |
| IS4 | 替换和清空       | 整章替换重新生成 ID；允许目标子集；空 pages 或空 issues 清空监稿并保留成稿页                  |
| IS5 | PSD ZIP 确认     | 分配、新版本确认及重试均保留监稿                                                              |
| IS6 | 发布与图源独立   | Page 删除保留监稿；发布后拒绝监稿导入、成稿清单、图片替换及确认                               |
| IS7 | 成稿重排和替换   | 显式 ID 重排复用已上传图片；改名及图片新版本保留监稿；旧版本确认失败                          |
| IS8 | 成稿删除与事务   | 未知成稿 ID 拒绝并保留监稿；省略旧成稿删除其监稿；空清单清空成稿页                            |

Rust RDB 测试补充数据库约束、批量插入、替换失败回滚、并发批次隔离和归档快照；Mock 用例覆盖相同业务权限、替换和清理规则。
