# Delete Cascade Side-Effect Cleanup Plan

This plan fixes delete cascade orchestration without removing database
`ON DELETE CASCADE` constraints. Database cascade remains the row-level
referential-integrity guarantee. The business requirement is that every
side-effect cleanup intention is recorded in `prom` before the database row
delete that may trigger cascade.

## Goal

Delete flows must be transactional and complete:

1. Lock and read the root entity.
2. Enumerate every descendant row that can carry side-effect resources.
3. Append `prom` records for those side-effect resources.
4. Delete the root row last.
5. Let database `ON DELETE CASCADE` remove descendant rows as the physical row
   cleanup guarantee.

The side-effect resources are not deleted directly inside the transaction.
The transaction writes local prom records first, then the prom worker performs
the external cleanup after commit.

## Non-Goals

- Do not remove database `ON DELETE CASCADE` constraints.
- Do not move object-storage operations into the transaction.
- Do not make repo implementations responsible for business cleanup.
- Do not add repo methods that are named or documented as business cascade.

## Current Problems

- `WorksetStep::DeleteCascade` exposes business cascade terminology at the
  repo step layer.
- `workset::delete` delegates directly to a repo cascade step and has no prom
  cleanup path for child resources.
- `team::delete` deletes worksets through the workset cascade step, so nested
  comic cover cleanup is not guaranteed before the team delete triggers
  database cascade.
- `comic::delete` already notes that `ComicComplex::delete_cascade` is missing.
  It only handles the comic cover and does not yet provide a reusable cascade
  orchestration point.
- The mock `DeleteCascade` behavior does not model production database cascade
  accurately, which can hide missing side-effect cleanup in tests.

## Layer Contract

### Complex

Complex owns business delete cascade.

For each aggregate delete cascade method:

- Read or lock the root entity before deleting it.
- List the descendants that can be removed by database cascade.
- Append prom records for every side-effect resource before deleting the root.
- Call repo delete only after all prom records for the affected subtree have
  been appended.

### Repo

Repo owns row operations only.

Allowed operations:

- `get_info_excluded`
- `list_by_parent_id_excluded`
- `delete`
- `delete_by_parent_id` only when it means a plain row-level delete and has no
  side-effect responsibilities

Forbidden operations:

- `delete_cascade`
- `DeleteCascade`
- Any repo method that appends prom records or understands avatar, cover, or
  page image cleanup.

### Database

Database `ON DELETE CASCADE` remains valid and useful. It should be treated as
the final physical row cleanup after Complex has recorded all business
side-effect cleanup intentions.

## Execution Steps

### 1. Inventory Cascade Paths

List every FK cascade path currently present in migrations and schema.

Initial known paths:

- `t_team -> t_workset`
- `t_workset -> t_comic`
- `t_user -> t_member`
- `t_team -> t_member`
- `t_user -> t_member_invitation`
- `t_team -> t_member_invitation`
- `t_user -> t_system_mail`

For each path, mark whether descendants may contain side-effect resource keys
now or in planned nearby work.

### 2. Rename Repo Cascade Surface

Replace workset repo cascade naming with row-level naming:

- `DeleteCascade` -> `Delete`
- `WorksetStep::delete_cascade` -> `WorksetStep::delete`
- `Advance<DeleteCascade, C>` -> `Advance<Delete, C>`

The implementation may delete one workset row and rely on DB cascade for child
rows. The repo must not append prom records or enumerate business descendants.

### 3. Implement Complex Orchestration

Add delete cascade functions to Complex where the business workflow belongs.

Required initial functions:

- `ComicComplex::delete_cascade`
- `WorksetComplex::delete_cascade`
- `TeamComplex::delete_cascade`

Each function receives the transactional repo handle, prom handle, and
transaction context required by the current `Advance` architecture.

Ordering rules:

- Comic delete:
  - Lock/read comic.
  - Enumerate chapter/page resources when those repos exist.
  - Append cover-delete prom when the comic has a cover key that must be
    cleaned.
  - Delete the comic row.
- Workset delete:
  - Lock/read workset if needed.
  - List comics under the workset before deleting the workset.
  - For each comic, run the comic side-effect preparation path.
  - Delete the workset row last, letting DB cascade remove comic rows.
- Team delete:
  - Lock/read team.
  - List worksets under the team before deleting the team.
  - For each workset, run the workset side-effect preparation path.
  - Append avatar-delete prom for the team avatar when needed.
  - Delete the team row last, letting DB cascade remove worksets and children.

When child repos are not implemented yet, leave the traversal points explicit
and covered by tests for currently implemented child resource types.

### 4. Update Usecases

Make usecase delete functions transaction shells:

- Open transaction with `Drive::with_context`.
- Derive transactional repo and prom handles.
- Call the relevant Complex delete cascade function.
- Do not inline side-effect cleanup in the usecase.
- Do not call repo cascade methods.

Affected usecases:

- `usecase::comic::delete`
- `usecase::workset::delete`
- `usecase::team::delete`
- `usecase::user::delete` if it can be normalized without expanding scope too
  far

### 5. Align Mocks

Mocks must support tests that distinguish these responsibilities:

- Complex appends prom records before deleting the root row.
- Repo plain delete has no business cleanup.
- Repo plain delete may model DB cascade row removal where production DB would
  cascade.

Do not keep a mock-only `DeleteCascade` behavior.

### 6. Expand Tests

Add or update tests for these cases:

- Deleting a comic with a cover appends exactly one image-delete prom record
  before the comic is gone.
- Deleting a workset with multiple covered comics appends one image-delete prom
  record per affected comic cover.
- Deleting a team with an avatar and covered child comics appends prom records
  for both the team avatar and descendant comic covers.
- Calling the row-level repo delete directly does not create prom records.
- Missing root entity rolls back and does not append prom records.

Keep the existing positive and negative usecase test minimums.

### 7. Verification

Run the normal verification workflow:

```text
cargo check
cargo test
```

Also grep for forbidden business-cascade repo surfaces:

```text
rg -n "DeleteCascade|delete_cascade|cascade-delete|cascade delete" src/part src/part_impl src/usecase src/complex
```

Any remaining match must be either a Complex-level business cascade function or
a test description explicitly validating the Complex behavior.

## Acceptance Criteria

- Database cascade constraints remain in migrations.
- No repo step or repo trait exposes `DeleteCascade` or `delete_cascade`.
- Delete usecases delegate business cleanup ordering to Complex.
- Complex appends all known side-effect delete prom records before issuing the
  row delete that triggers database cascade.
- Tests prove descendant side-effect cleanup for current implemented
  descendants.
- `cargo check` and relevant tests pass.
