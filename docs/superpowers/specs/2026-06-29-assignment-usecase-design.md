# Assignment Use Case Design

## Scope

Implement the active Rust assignment use case as a current-architecture vertical slice.
The feature covers the business behavior from the Go reference, but uses the Rust
ports-and-transaction-steps style.

Public use cases:

- `list_infos`
- `update_roles`

There is no public assignment `delete` use case. Deletion is represented as
`update_roles` with an empty role mask. The use case dispatches that case to a
repository delete step inside the same aggregate endpoint.

## Architecture

Assignment follows the same shape as `member::list_infos`:

- `ListAssignmentInfosData` converts into `AssignmentListSpec`.
- `AssignmentStep::list_infos(&AssignmentListSpec)` returns `Vec<AssignmentInfo>`.
- `assignment::list_infos` performs permission checks from the selected list spec,
  executes the list step, and maps models into `AssignmentInfoVal`.

Role mutation follows one use case:

- `UpdateAssignmentRoleData` carries `chapter_id`, `user_id`, and `roles`.
- `assignment::update_roles` resolves authorization and target eligibility before
  mutation.
- Non-empty roles create a missing assignment or overwrite the current role mask.
- Empty roles delete the existing assignment if present, and are idempotent when no
  assignment exists.

The repository layer still has explicit delete support, but it is only used by
`update_roles` and internal cascade flows.

## Data Model

Add an assignment list spec to `src/model/assignment.rs`:

- `AssignmentListSpec::Chapter { chapter_id, role_bit, offset, limit }`
- `AssignmentListSpec::User { owner_id, role_bit, offset, limit }`

Add assignment DTOs in `src/data/assignment.rs`:

- `AssignmentInfoVal`, equivalent to the current chapter DTO assignment value.
- `ListAssignmentInfosData`, using `#[Paginate]`, with exactly one of
  `chapter_id` or `owner_id`.
- `UpdateAssignmentRoleData`, with `chapter_id`, `user_id`, and `roles`.

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
- Non-empty target roles must not include `ADMIN`.
- Non-empty target roles require the target user to be a member of the chapter's
  owning team, and the target member role mask must contain the requested assignment
  role mask.
- Empty roles skip target membership eligibility and delete if an assignment exists.

## Repository Steps

Extend assignment steps:

- `ListInfos`
- `GetInfoById`
- `GetInfoByIdExcluded`
- `Delete`

Keep existing steps:

- `GetInfoByChapterUserId`
- `Create`
- `PutRoles`

`PutRoles` overwrites the assignment role mask. It does not merge; merge behavior is
handled by callers such as `chapter::join`.

## Mock Behavior

The assignment mock repository should:

- List by chapter or owner.
- Filter by role bit when provided.
- Sort deterministically by creation order, matching the existing member mock style.
- Apply offset and limit after filtering.
- Return expected errors for missing `get` or `delete` targets.
- Treat delete-by-chapter-user inside `update_roles` as idempotent at the use case
  level, not as an idempotent raw repository delete.

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
- Empty role update deletes an existing assignment.
- Empty role update is idempotent when missing.

## Non-Goals

- No public `delete` use case.
- No nested include graph or signed comic cover filling.
- No Go-shaped service or event bus port.
- No legacy HTTP or Diesel implementation work.
