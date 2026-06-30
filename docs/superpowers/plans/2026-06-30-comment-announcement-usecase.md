# Comment Announcement Usecase Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement active-architecture comment and announcement use cases with standard `incl` behavior and mock repository support.

**Architecture:** Add complete vertical slices for comment and announcement without real Diesel or HTTP implementations. List operations are non-transactional and parameterized by list specs containing `incl_opt`; create operations run through `Drive::with_context`.

**Tech Stack:** Rust 2024, Tokio async tests, `poprako_transactional`, project mock repository, `poprako_macro::Paginate`, `serde::Deserialize`.

---

## File Structure

- Create `src/value/comment.rs`: `CommentInclOpt`.
- Create `src/value/announcement.rs`: `AnnouncementInclOpt`.
- Modify `src/value.rs`: register new value modules.
- Create `src/model/comment.rs`: `CommentInfo`, `CommentForm`, `CommentListSpec`.
- Create `src/model/announcement.rs`: `AnnouncementInfo`, `AnnouncementForm`, `AnnouncementListSpec`.
- Modify `src/model.rs`: register new model modules.
- Create `src/data/comment.rs`: comment data and val DTOs.
- Create `src/data/announcement.rs`: announcement data and val DTOs.
- Modify `src/data.rs`: register new data modules.
- Create `src/complex/comment.rs`: ID generation and comment permission helpers.
- Create `src/complex/announcement.rs`: ID generation and announcement permission helpers.
- Modify `src/complex.rs`: register new complex modules.
- Create `src/part/repo/step/comment.rs`: `ListInfos`, `Create`, `CommentStep`.
- Create `src/part/repo/step/announcement.rs`: `ListInfos`, `Create`, `AnnouncementStep`.
- Modify `src/part/repo/step.rs`: register new step modules.
- Create `src/part/repo/comment.rs`: repo traits.
- Create `src/part/repo/announcement.rs`: repo traits.
- Modify `src/part/repo.rs`: register new repo modules.
- Modify `src/part_impl/repo_mock.rs`: add state, snapshot, and seed helpers.
- Create `src/part_impl/repo_mock/comment.rs`: mock repo implementation and repo tests.
- Create `src/part_impl/repo_mock/announcement.rs`: mock repo implementation and repo tests.
- Modify `src/part_impl.rs`: register new mock modules if the root lists repo mock submodules there.
- Modify `src/usecase/comment.rs`: implement public use cases.
- Modify `src/usecase/announcement.rs`: implement public use cases.
- Create `src/usecase/comment/tests.rs`: comment use case tests.
- Create `src/usecase/announcement/tests.rs`: announcement use case tests.
- Modify `poprako-util/locales/en-US/main.ftl`: add not-found keys.
- Modify `poprako-util/locales/zh-CN/main.ftl`: add not-found keys.

## Required Project Skills During Execution

- Read `general-conventions` before editing Rust source.
- Read `rust-use-style` before editing imports.
- Read `rust-ident-style` before editing call-site paths.
- Read `format-output-spec` or `no-inline-format` before adding format strings.
- Read `test-spec` before adding tests.
- Use `verification-before-completion` before claiming completion.

## Task 1: Value, Model, And Data Types

**Files:**

- Create: `src/value/comment.rs`
- Create: `src/value/announcement.rs`
- Modify: `src/value.rs`
- Create: `src/model/comment.rs`
- Create: `src/model/announcement.rs`
- Modify: `src/model.rs`
- Create: `src/data/comment.rs`
- Create: `src/data/announcement.rs`
- Modify: `src/data.rs`

- [ ] **Step 1: Add failing type-check target**

Run:

```bash
cargo check
```

Expected: current baseline may pass or fail because unrelated page files are dirty. Record any pre-existing failure before editing.

- [ ] **Step 2: Add include option modules**

`src/value/comment.rs`:

```rust
//! Value types for comment aggregates.

use serde::Deserialize;

/// Include options for comment info queries.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum CommentInclOpt {
    User,
}
```

`src/value/announcement.rs`:

```rust
//! Value types for announcement aggregates.

use serde::Deserialize;

/// Include options for announcement info queries.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum AnnouncementInclOpt {
    User,
}
```

Register both modules in `src/value.rs`:

```rust
pub mod announcement;
pub mod comment;
```

- [ ] **Step 3: Add model types**

Create `src/model/comment.rs` with:

```rust
//! Domain models for team board comments.

use time::OffsetDateTime;

use crate::model::user::UserInfo;
use crate::value::comment::CommentInclOpt;

#[cfg_attr(test, derive(Clone))]
pub struct CommentInfo {
    pub id: String,

    pub team_id: String,
    pub user_id: String,
    pub user: Option<UserInfo>,

    pub content: String,

    pub created_at: OffsetDateTime,
}

#[cfg_attr(test, derive(Clone))]
pub struct CommentForm {
    pub id: String,

    pub team_id: String,
    pub user_id: String,

    pub content: String,
}

pub struct CommentListSpec {
    pub team_id: String,
    pub incl_opt: Vec<CommentInclOpt>,
    pub offset: u64,
    pub limit: u64,
}
```

Create `src/model/announcement.rs` with:

```rust
//! Domain models for team announcements.

use time::OffsetDateTime;

use crate::model::user::UserInfo;
use crate::value::announcement::AnnouncementInclOpt;

#[cfg_attr(test, derive(Clone))]
pub struct AnnouncementInfo {
    pub id: String,

    pub team_id: String,
    pub user_id: String,
    pub user: Option<UserInfo>,

    pub title: String,
    pub content: String,

    pub created_at: OffsetDateTime,
}

#[cfg_attr(test, derive(Clone))]
pub struct AnnouncementForm {
    pub id: String,

    pub team_id: String,
    pub user_id: String,

    pub title: String,
    pub content: String,
}

pub struct AnnouncementListSpec {
    pub team_id: String,
    pub incl_opt: Vec<AnnouncementInclOpt>,
    pub offset: u64,
    pub limit: u64,
}
```

Register both modules in `src/model.rs`.

- [ ] **Step 4: Add data DTOs**

Create DTO modules with `#[Paginate]`, `Deserialize`, `InfoVal::from_model`, and `CreateXxxVal { id }`. `InfoVal::from_model` must map included users through `UserInfoVal::from_model(image_pool, user_info).await?`.

Run:

```bash
cargo check
```

Expected: failures for missing complex, repository, and use case modules until Tasks 2 through 5 are complete.

## Task 2: Permission Complexes

**Files:**

- Create: `src/complex/comment.rs`
- Create: `src/complex/announcement.rs`
- Modify: `src/complex.rs`

- [ ] **Step 1: Add comment complex**

Implement `CommentComplex::gen_id()` with `next_snowflake_id()`.

Implement `CommentPermComplex`:

```rust
pub async fn can_user_list_infos<P>(proxy: &mut P, user_id: &str, team_id: &str) -> RootResult<()>
where
    P: for<'a> ProxyExecute<FindInfoByUserIdAndTeamId<'a>, Error = RootError>,
{
    check_user_is_team_member(proxy, user_id, team_id).await
}
```

Add `can_user_create` with the same member check.

- [ ] **Step 2: Add announcement complex**

Implement `AnnouncementComplex::gen_id()` with `next_snowflake_id()`.

Implement `AnnouncementPermComplex::can_user_list_infos` with team-member check and `can_user_create` with team-admin check.

- [ ] **Step 3: Register modules and check**

Register modules in `src/complex.rs`.

Run:

```bash
cargo check
```

Expected: failures for missing repo modules until Task 3.

## Task 3: Repository Steps And Traits

**Files:**

- Create: `src/part/repo/step/comment.rs`
- Create: `src/part/repo/step/announcement.rs`
- Modify: `src/part/repo/step.rs`
- Create: `src/part/repo/comment.rs`
- Create: `src/part/repo/announcement.rs`
- Modify: `src/part/repo.rs`

- [ ] **Step 1: Add step modules**

Each step module defines `ListInfos<'a>`, `Create<'a>`, and a factory struct. `ListInfos` outputs `Vec<XxxInfo>`; `Create` outputs `XxxInfo`.

- [ ] **Step 2: Add repo traits**

`CommentRepo<C>` extends `DeriveTransactional + Execute<ListInfos<'a>, Error = RootError>`.

`CommentRepoTransactional<C>` extends `Advance<Create<'a>, C, Error = RootError> + Sized`.

Apply the same shape to announcement.

- [ ] **Step 3: Register modules and check**

Run:

```bash
cargo check
```

Expected: mock and use case compile failures until Tasks 4 and 5 are complete.

## Task 4: Mock Repository State And Implementations

**Files:**

- Modify: `src/part_impl/repo_mock.rs`
- Create: `src/part_impl/repo_mock/comment.rs`
- Create: `src/part_impl/repo_mock/announcement.rs`
- Modify: `src/part_impl.rs`

- [ ] **Step 1: Extend mock state**

Add `comments: Vec<CommentInfo>` and `announcements: Vec<AnnouncementInfo>` to `MockState` and `MockSnapshot`.

Add `seed_comment` and `seed_announcement` inherent methods on `Mock`.

- [ ] **Step 2: Implement comment mock**

Implement:

- `impl CommentRepo<MockContext> for Mock {}`
- `impl CommentRepoTransactional<MockContext> for MockTransactional {}`
- `Execute<ListInfos<'a>> for Mock`
- `Advance<Create<'a>, MockContext> for MockTransactional`

List helper requirements:

- Filter by `team_id`.
- Sort by `created_at` descending.
- Page after sorting.
- Set `user` only when `spec.incl_opt.contains(&CommentInclOpt::User)`.

- [ ] **Step 3: Implement announcement mock**

Mirror comment mock behavior with announcement types and `AnnouncementInclOpt::User`.

- [ ] **Step 4: Add mock repo tests**

Test descriptions must be module-level comments. Cover list filtering, sorting, pagination, include on/off, create, and duplicate rejection for both domains.

Run:

```bash
cargo test -p poprako-r part_impl::repo_mock::comment
cargo test -p poprako-r part_impl::repo_mock::announcement
```

Expected: both targeted mock test groups pass.

## Task 5: Use Case Implementations

**Files:**

- Modify: `src/usecase/comment.rs`
- Modify: `src/usecase/announcement.rs`
- Create: `src/usecase/comment/tests.rs`
- Create: `src/usecase/announcement/tests.rs`

- [ ] **Step 1: Implement comment use cases**

`comment::list_infos`:

- Convert data into `CommentListSpec`.
- Use `repo.as_proxy()` and `CommentPermComplex::can_user_list_infos`.
- Execute `CommentStep::list_infos`.
- Convert with `CommentInfoVal::from_model(image_pool, comment_info).await?`.

`comment::create`:

- Use permission check before transaction.
- Build `CommentForm` as a named local.
- Run `drive.with_context`.
- Advance `CommentStep::create`.
- Return `CreateCommentVal { id: comment_info.id }`.

- [ ] **Step 2: Implement announcement use cases**

Mirror comment list behavior with announcement types.

For create, use `AnnouncementPermComplex::can_user_create`, which requires team admin.

- [ ] **Step 3: Add use case tests**

Use `Mock` as drive, repo, and image pool. Seed users and members with role masks.

Required positive and negative coverage:

- Comment list member success.
- Comment list non-member permission error.
- Comment list include user on and off.
- Comment create member success.
- Comment create non-member permission error and no mutation.
- Announcement list member success.
- Announcement list non-member permission error.
- Announcement list include user on and off.
- Announcement create admin success.
- Announcement create non-admin member permission error and no mutation.
- Announcement create non-member permission error and no mutation.

Run:

```bash
cargo test -p poprako-r usecase::comment
cargo test -p poprako-r usecase::announcement
```

Expected: both targeted use case test groups pass.

## Task 6: Locale Keys And Verification

**Files:**

- Modify: `poprako-util/locales/en-US/main.ftl`
- Modify: `poprako-util/locales/zh-CN/main.ftl`

- [ ] **Step 1: Add locale keys**

English:

```text
error-comment-not-found = Comment not found
error-announcement-not-found = Announcement not found
```

Chinese:

```text
error-comment-not-found = 留言不存在
error-announcement-not-found = 公告不存在
```

- [ ] **Step 2: Format and verify**

Run:

```bash
cargo fmt
cargo test -p poprako-r usecase::comment
cargo test -p poprako-r usecase::announcement
cargo test -p poprako-r part_impl::repo_mock::comment
cargo test -p poprako-r part_impl::repo_mock::announcement
cargo check
```

Expected: targeted tests pass. `cargo check` should pass unless unrelated dirty page changes introduce failures; if so, record the exact unrelated failures and confirm the new comment/announcement code compiles in targeted checks.

- [ ] **Step 3: Style verification**

Run:

```bash
just style
```

Expected: style checks pass. If unrelated dirty files fail style, record the file and rule without modifying those unrelated changes.
