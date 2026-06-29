# Assignment Use Case Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to
> implement this plan task-by-task. Keep the non-zero `RoleMask` invariant intact.

**Goal:** Implement active Rust assignment listing, role updates, and public delete.
`update_roles` requires `roles: RoleMask`; it must not accept missing roles and
must not delete assignments. Deletion is handled only by public `delete`.

**Architecture:** Assignment follows the current ports-and-transaction-steps
application core. `list_infos` mirrors `member::list_infos` through a
`Data -> ListSpec -> Step -> Val` flow. `update_roles` creates or overwrites roles
in one transaction. `delete` loads by id, checks permission, then deletes by id
transactionally.

**Tech Stack:** Rust 2024, Tokio tests, `poprako-transactional`, in-memory mock
repo ports.

---

### Task 1: Add Assignment Data And List Spec

**Files:**

- Modify: `src/model/assignment.rs`
- Create: `src/data/assignment.rs`
- Modify: `src/data.rs`
- Test through: `src/usecase/assignment/tests/*`

- [ ] Add `AssignmentListSpec::{Chapter, User}` with `role_bit`, `offset`, and
  `limit`.
- [ ] Add `AssignmentInfoVal`.
- [ ] Add `ListAssignmentInfosData` with exactly one of `chapter_id` or
  `owner_id`.
- [ ] Add `UpdateAssignmentRoleData { chapter_id, user_id, roles: RoleMask }`.
- [ ] Export `data::assignment`.

### Task 2: Extend Assignment Repository Steps And Mock

**Files:**

- Modify: `src/part/repo/step/assignment.rs`
- Modify: `src/part/repo/assignment.rs`
- Modify: `src/part_impl/repo_mock/assignment.rs`

- [ ] Add production-used steps: `ListInfos`, `GetInfoById`, `Delete`.
- [ ] Keep existing steps: `GetInfoByChapterUserId`, `Create`, `PutRoles`.
- [ ] Add `Execute<ListInfos>` and `Execute<GetInfoById>` to `AssignmentRepo<C>`.
- [ ] Add `Advance<Delete>` to `AssignmentRepoTransactional<C>`.
- [ ] Keep repository traits limited to production-used steps.
- [ ] Implement mock list filtering by chapter or owner, optional `role_bit`,
  deterministic id sorting, offset, and limit.
- [ ] Implement mock delete by id.

### Task 3: Implement Permissions And Usecases

**Files:**

- Modify: `src/complex/assignment.rs`
- Modify: `src/usecase/assignment.rs`
- Test through: `src/usecase/assignment/tests/*`

- [ ] Add `AssignmentPermComplex::can_user_list_infos`.
- [ ] Add `AssignmentPermComplex::can_user_update_roles`.
- [ ] Add `AssignmentPermComplex::can_user_delete`.
- [ ] Implement `list_infos`: data conversion, permission check, list step, map
  to vals.
- [ ] Implement `update_roles`: permission check, get existing assignment, create
  missing assignment, overwrite existing assignment.
- [ ] Implement `delete`: get by id, permission check, transactional delete by id.

### Task 4: Tests

**Files:**

- `src/usecase/assignment/tests/list_infos.rs`
- `src/usecase/assignment/tests/update_roles.rs`
- `src/usecase/assignment/tests/delete.rs`

- [ ] Cover chapter list by team membership.
- [ ] Cover chapter list by existing assignment fallback.
- [ ] Cover owner and super-admin user list.
- [ ] Cover rejected chapter/user list.
- [ ] Cover reviewer create and overwrite.
- [ ] Cover self role reduction and rejected self expansion.
- [ ] Cover rejected non-reviewer update of another user.
- [ ] Cover rejected `ADMIN` assignment.
- [ ] Cover rejected target member role mismatch.
- [ ] Cover owner delete, reviewer delete, and rejected non-reviewer delete.

### Task 5: Verification

- [ ] Run `cargo fmt`.
- [ ] Run `cargo test -p poprako-r usecase::assignment::tests`.
- [ ] Run `cargo check`.
- [ ] Run `cargo test -p poprako-r` when feasible.
- [ ] Run Rust use-style and ident-style checks on touched Rust files.
- [ ] Confirm no touched Rust file exceeds 600 lines.
- [ ] Confirm assignment role updates always carry `roles: RoleMask`.
