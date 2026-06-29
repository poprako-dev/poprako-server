# Assignment Use Case Design

## Scope

Implement the active Rust assignment use case as a current-architecture vertical slice.
The feature covers the reference business behavior while using the Rust
ports-and-transaction-steps style.

Public use cases:

- `list_infos`
- `update_roles`
- `delete`

`update_roles` requires a valid non-zero `RoleMask`. It does not accept a missing
roles value and does not delete assignments. Deletion is only exposed through the
public `delete` use case.

## Architecture

Assignment follows the same shape as `member::list_infos`:

- `ListAssignmentInfosData` converts into `AssignmentListSpec`.
- `AssignmentStep::list_infos(&AssignmentListSpec)` returns `Vec<AssignmentInfo>`.
- `assignment::list_infos` performs permission checks from the selected list spec,
  executes the list step, and maps models into `AssignmentInfoVal`.

Role mutation follows one use case:

- `UpdateAssignmentRoleData` carries `chapter_id`, `user_id`, and `roles: RoleMask`.
- `assignment::update_roles` resolves authorization and target eligibility before
  mutation.
- Roles create a missing assignment or overwrite the current role mask.

Public deletion:

- `assignment::delete` loads the target by identifier.
- Ownership permits deleting a caller's own assignment.
- Deleting another user's assignment requires reviewer role in the chapter.
- The actual delete runs inside a transaction.

## Data Model

Add an assignment list spec to `src/model/assignment.rs`:

- `AssignmentListSpec::Chapter { chapter_id, role_bit, offset, limit }`
- `AssignmentListSpec::User { owner_id, role_bit, offset, limit }`

Add assignment DTOs in `src/data/assignment.rs`:

- `AssignmentInfoVal`, equivalent to the current chapter DTO assignment value.
- `ListAssignmentInfosData`, using `#[Paginate]`, with exactly one of
  `chapter_id` or `owner_id`.
- `UpdateAssignmentRoleData`, with `chapter_id`, `user_id`, and `roles: RoleMask`.

The list data follows `member::list_infos`: invalid combinations are rejected during
conversion into the list spec. `chapter_id` and `owner_id` are mutually exclusive.

## Permissions

Assignment permission helpers live in `AssignmentPermComplex`.

`list_infos`:

- Chapter-scoped listing is allowed when the caller is a member of the owning team.
- If the team-member path fails, chapter-scoped listing is also allowed when the
  caller already has an assignment on the chapter.
- User-scoped listing is allowed for the owner themself or a super admin.

`update_roles`:

- If the caller updates another user, the caller must hold the chapter reviewer role.
- If the caller updates themself, reviewer permission allows any valid mutation.
- A non-reviewer self-update may only remove existing roles or reduce the role mask.
  It must not add new role bits.
- Target roles must not include `ADMIN`.
- Target roles require the target user to be a member of the chapter's owning team,
  and the target member role mask must contain the requested assignment role mask.

`delete`:

- Deleting an owned assignment is allowed.
- Deleting another user's assignment requires reviewer role in the chapter.

## Repository Steps

Extend assignment steps:

- `ListInfos`
- `GetInfoById`
- `Delete`

Keep existing steps:

- `GetInfoByChapterUserId`
- `Create`
- `PutRoles`

Keep repository traits limited to the steps used by production use cases. Traits
must not be expanded for test-only convenience.

`PutRoles` overwrites the assignment role mask. It does not merge; merge behavior is
handled by callers such as `chapter::join`.

## Mock Behavior

The assignment mock repository should:

- List by chapter or owner.
- Filter by role bit when provided.
- Sort deterministically by assignment id.
- Apply offset and limit after filtering.
- Return expected errors for delete-by-id targets that are missing.

## Tests

Add focused use case tests for:

- Chapter list allowed by team membership.
- Chapter list allowed by existing assignment fallback.
- Chapter list rejected for unrelated users.
- User list allowed by owner.
- User list allowed by super admin.
- User list rejected for non-owner non-admin.
- Role update by reviewer creates missing assignment.
- Role update by reviewer overwrites existing assignment roles.
- Self role reduction succeeds.
- Self role expansion is rejected.
- Non-reviewer updating another user is rejected.
- Assigning `ADMIN` is rejected.
- Assigning a role the target member cannot hold is rejected.
- Owner can delete their own assignment.
- Reviewer can delete another user's assignment.
- Non-reviewer cannot delete another user's assignment.

## Non-Goals

- No deletion through `update_roles`.
- Assignment role updates always carry `roles: RoleMask`.
- No nested include graph or signed comic cover filling.
- No Go-shaped service or event bus port.
- No legacy HTTP or Diesel implementation work.
