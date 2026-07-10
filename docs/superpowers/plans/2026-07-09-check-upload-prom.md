# Check Upload Prom Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `ImageTask::CheckUploaded` mark successful uploads in the repository and guarantee that missing-owner objects cannot leak while stale versions converge through ordered delete messages.

**Architecture:** Keep `Prom<C>` as the transaction-local append port, but make the RDB prom consumer a real durable queue. `CheckUploaded` becomes an ownership reconciliation task: if the object exists and the DB still points at the same `resource_id + version`, mark uploaded; if the DB points at a newer version, complete and rely on the earlier ordered `Delete(prev_key)` message emitted by the reserve transaction; if the owner resource no longer exists, delete the orphan object. External failures requeue instead of going dead.

**Tech Stack:** Rust 2024, `poprako_transactional::{Drive, Advance, Step}`, Diesel async, `ImagePool`.

---

## Requirements Summary

- `CheckUploaded` must call the matching repo mark step when `head_object(object_key)` returns true.
- Version mismatch or missing resource must not become an infinite retry.
- A stale-but-owned object is left to the ordered delete task; an object whose owning resource is missing must be deleted through a retryable delete path.
- `head_object` and `delete_object` transient failures must be retried.
- Worker crashes must not leave messages permanently in `Processing`.
- Before every claim round, timed-out `Processing` records must be made claimable again; after more than three processing timeouts, they become `Dead`.
- Archive behavior is out of scope.

## State Machine

### CheckUploaded

- `head_object == true`, repo mark succeeds:
  - Mark the resource uploaded inside a transaction.
  - Complete the prom record.

- `head_object == true`, repo mark returns version mismatch:
  - The upload object exists but DB now owns another version of the same resource.
  - Complete the prom record.
  - Do not delete here; ordered consumption guarantees the reserve transaction's `Delete(prev_key)` task handles stale object cleanup before this check is consumed.

- `head_object == true`, owner resource is missing:
  - Delete `object_key`.
  - If delete succeeds, complete the prom record.
  - If delete fails, mark the prom record pending with `visible_at = now + retry_delay`.

- `head_object == false`:
  - No object was uploaded.
  - Complete the prom record.
  - No delete is needed because there is no remote object to leak.
  - DB remains unuploaded; the next reservation version will replace it.

- `head_object` returns external error:
  - Mark pending with last error and retry delay.
  - Do not complete and do not mark uploaded.

- Repo mark returns unrecoverable DB/transaction error:
  - Mark pending with retry delay.
  - Do not delete yet because deleting after an uncertain DB failure risks deleting the only uploaded object before DB can be marked.

### Delete

- `delete_object` succeeds:
  - Complete the prom record.

- `delete_object` fails:
  - Mark pending with last error and retry delay.

### Queue Maintenance

- `Pending -> Processing` on claim.
- `Processing -> Completed` on success.
- `Processing -> Pending` on retryable failure, incrementing retry count and setting next `visible_at`.
- Stuck `Processing` records older than processing timeout reset to `Pending`.
- Stuck `Processing` records that have timed out more than three times move to `Dead`.
- Old `Completed` records are purged.
- Malformed payloads and records that repeatedly time out beyond the timeout threshold move to `Dead`; retryable resource cleanup still prefers retry over leaking.

## Implementation Steps

### Task 1: Add Prom Queue Retry Steps

**Files:**
- Modify: `src/part_impl/prom/rdb_impl.rs`
- Modify: `src/part_impl/prom/rdb_impl/handler.rs`
- Modify: `src/part_impl/prom/mock_impl.rs`
- Modify: `src/part_impl/repo/mock_impl.rs`

- [x] Add internal steps in `src/part_impl/prom/rdb_impl.rs`:
  - `RetryStep { id, error, visible_at }`
  - `ResetStuckStep { before }`

- [x] Implement `Advance<RetryStep, RdbContext>`:
  - `status = Pending`
  - `f_last_error = Some(error)`
  - `f_visible_at = visible_at`
  - `f_retried_count = f_retried_count + 1`
  - `f_updated_at = now`

- [x] Implement `Advance<ResetStuckStep, RdbContext>`:
  - `Processing` records with `f_updated_at <= before` become `Pending`.
  - `Processing` records that have already timed out three times become `Dead`.
  - Leave `f_last_error` according to local style; do not delete rows.

- [x] Update `RdbPromHandler` bounds to require the new steps.

- [x] Add handler constants:
  - `RETRY_DELAY = Duration::from_secs(300)`
  - `PROCESSING_TIMEOUT = Duration::from_secs(900)`

- [x] In `run` and before each row claim, call `reset_stuck`.

### Task 2: Add CheckUploaded Marking Dispatch

**Files:**
- Modify: `src/part_impl/prom/rdb_impl/handler.rs`
- Modify: `src/part_impl/prom/rdb_impl/handler/image.rs`

- [x] Extend `image::handle` bounds so it can use transactional repo mark steps:
  - `UserRepoTransactional<RdbContext>`
  - `TeamRepoTransactional<RdbContext>`
  - `ComicRepoTransactional<RdbContext>`
  - `PageRepoTransactional<RdbContext>`

- [x] Add these repo traits to `RdbPromHandler` and `dispatch_topic` bounds.

- [x] Implement `mark_uploaded_by_kind` in `handler/image.rs`:
  - `ImageKind::UserAvatar` -> `UserStep::mark_avatar_uploaded(resource_id, image_version)`
  - `ImageKind::TeamAvatar` -> `TeamStep::mark_avatar_uploaded(resource_id, image_version)`
  - `ImageKind::ComicCover` -> `ComicStep::mark_cover_uploaded(resource_id, image_version)`
  - `ImageKind::PageImage` -> `PageStep::mark_image_uploaded(resource_id, image_version)`

- [x] Run the mark in `drive.with_context`.

- [x] Introduce a small queue outcome enum:
  - `TaskOutcome::Complete`
  - `TaskOutcome::Retry(String)`
  - `TaskOutcome::Dead(String)`

- [x] Classify repo `Expected` errors from mark steps as stale/missing:
  - Return `Complete` for stale owned versions and rely on ordered delete.
  - Delete missing-owner objects and return `Retry` if deletion fails.

- [x] Classify repo `Unrecoverable` errors as retryable:
  - Do not delete the object in this path.

### Task 3: Route Handler Outcomes To Queue Updates

**Files:**
- Modify: `src/part_impl/prom/rdb_impl/handler.rs`
- Modify: `src/part_impl/prom/rdb_impl/handler/image.rs`
- Modify: `src/part_impl/prom/rdb_impl/handler/comic.rs`

- [x] Change topic dispatch to return an internal queue outcome instead of plain `RegularResult<()>`.

- [x] For successful work, call `complete`.

- [x] For retryable work failures, call `retry(id, error, now + RETRY_DELAY)`.

- [x] Keep unknown topic and malformed payload as `Dead`.

- [x] Stop ignoring lifecycle update errors:
  - If `complete` fails, log it as an error with record id.
  - If `retry` fails, log it as an error with record id and original error.

### Task 4: Align Mock Prom Processor

**Files:**
- Modify: `src/part_impl/prom/mock_impl.rs`
- Modify: `src/part_impl/repo/mock_impl.rs`
- Test: `src/part_impl/prom/mock_impl.rs` or focused usecase tests

- [x] For `CheckUploaded` with existing object, call the matching mock repo mark step.

- [x] For stale/version mismatch with existing owner, leave deletion to ordered delete messages.

- [x] For missing owner with existing object, call `delete_object`.

### Task 5: Tests

**Files:**
- Add/modify tests near `src/part_impl/prom/mock_impl.rs`
- Add RDB tests only if the repo test harness is stable for prom; otherwise keep this as a documented gap.

- [x] Test `CheckUploaded` marks user avatar uploaded when object exists and version matches.

- [ ] Test the same for team avatar, comic cover, and page image.

- [ ] Test `head_object == false` completes without uploaded flag and without delete.

- [x] Test stale version with object present does not delete immediately and completes.

- [ ] Test stale version with delete failure retries instead of completing.

- [ ] Test head failure retries and does not mark uploaded.

- [ ] Test repo unrecoverable mark failure retries and does not delete.

- [ ] Test `Processing` stuck reset returns old records to pending.

- [ ] Test completed cleanup deletes old completed rows.

## Acceptance Criteria

- A successful `CheckUploaded` changes the matching `*_uploaded` column to true using version-checked repo steps.
- No existing remote object is abandoned after a stale or deleted DB resource path; it is deleted or retained in a retryable message.
- Transient storage or DB errors leave a retryable prom record.
- Worker crash during processing does not permanently hide the message.
- `cargo check` passes.
- Targeted prom/mock tests pass.

## Risks

- Existing repo mark steps report stale and missing through generic `Expected`; classifying all `Expected` as stale is acceptable for worker cleanup, but do not reuse that classification in user-facing APIs.
- Deleting after a stale mark is correct only because the object key embeds the version and cannot be the current DB-owned object once mark mismatches.
- If `delete_object` is not idempotent for missing objects, normalize missing-object delete to success in the image adapter.

## Verification

- Run `cargo fmt`.
- Run `cargo check`.
- Run targeted tests for `part_impl::prom`.
- Run affected usecase tests for `user`, `team`, `comic`, and `page`.
