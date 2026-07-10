# Comment And Announcement Use Case Design

## Scope

Implement the active Rust comment and announcement use cases as current-architecture
vertical slices. The feature covers model, value, data, complex, repository port,
repository step, mock repository, use case, use case tests, locale keys, and module
registration.

Public use cases:

- `comment::list_infos`
- `comment::create`
- `announcement::list_infos`
- `announcement::create`

The work does not implement real Diesel, HTTP, or legacy architecture adapters.

## Architecture

Comments and announcements follow the same ports-and-transaction-steps shape used
by the active use case layer:

- Data DTOs convert into list specs.
- List specs carry all query conditions, pagination, and include options.
- `XxxStep::list_infos(&XxxListSpec)` performs the list read.
- `XxxStep::create(&XxxForm)` performs the create write inside a transaction.
- Use cases perform permission checks before reads or writes.

Function names must stay standard. Conditions such as team scope are represented
only in data and spec fields, not in function names.

## Include Mechanism

The implementation must use the standard `incl` mechanism, not an implicit
hard-coded user preload.

New value types:

- `CommentInclOpt::User`
- `AnnouncementInclOpt::User`

New list specs:

- `CommentListSpec { team_id, incl_opt, offset, limit }`
- `AnnouncementListSpec { team_id, incl_opt, offset, limit }`

New info models carry optional included resources:

- `CommentInfo { id, team_id, user_id, user, content, created_at }`
- `AnnouncementInfo { id, team_id, user_id, user, title, content, created_at }`

The `user` field is populated only when `incl_opt` contains `User`. Without that
include option, the field must be `None`. This rule applies in the mock repository
and must be preserved by future real repository implementations.

## Data Model

Comment model types:

- `CommentInfo`
- `CommentForm`
- `CommentListSpec`

Announcement model types:

- `AnnouncementInfo`
- `AnnouncementForm`
- `AnnouncementListSpec`

Inbound DTOs:

- `ListCommentInfosData`
- `CreateCommentData`
- `ListAnnouncementInfosData`
- `CreateAnnouncementData`

Outbound DTOs:

- `CommentInfoVal`
- `CreateCommentVal`
- `AnnouncementInfoVal`
- `CreateAnnouncementVal`

`InfoVal` types preserve optional included user data by mapping `Option<UserInfo>`
into `Option<UserInfoVal>`. User values resolve avatar URLs through `ImagePool`,
following existing `UserInfoVal::from_model` behavior.

## Permissions

Comment permissions live in `CommentPermComplex`:

- `can_user_list_infos` requires the caller to be a member of the target team.
- `can_user_create` requires the caller to be a member of the target team.

Announcement permissions live in `AnnouncementPermComplex`:

- `can_user_list_infos` requires the caller to be a member of the target team.
- `can_user_create` requires the caller to be an admin of the target team.

Both permission complexes use the existing shared complex helpers for team member
and team admin checks. Complex modules stay pure with respect to transaction
ownership: they use proxy execution for permission reads and do not own
`Drive::with_context`.

## Repository Steps

Comment steps:

- `ListInfos`
- `Create`

Announcement steps:

- `ListInfos`
- `Create`

Repository traits expose only the active use case surface:

- `CommentRepo<C>` supports non-transactional `ListInfos`.
- `CommentRepoTransactional<C>` supports transactional `Create`.
- `AnnouncementRepo<C>` supports non-transactional `ListInfos`.
- `AnnouncementRepoTransactional<C>` supports transactional `Create`.

Create steps return the created info model. List steps return `Vec<XxxInfo>`.

## Mock Behavior

The mock repository state adds `comments` and `announcements`.

List behavior:

- Filter by `team_id`.
- Sort by `created_at` descending.
- Apply `offset` and `limit` after sorting.
- Populate optional user includes only when the list spec requests `User`.

Create behavior:

- Reject duplicate identifiers with `error-already-exists`.
- Insert a new row with the current timestamp.
- Return the created info with `user: None`.
- Commit or roll back through the existing mock transaction driver.

Mock fixtures gain `seed_comment` and `seed_announcement` helpers plus snapshot
fields for assertions.

## Locale Keys

Add not-found keys for mock and future repository consistency:

- `error-comment-not-found`
- `error-announcement-not-found`

The initial public use cases do not fetch by id, but repository mocks should have
domain-specific keys available for future tests or steps.

## Tests

Add focused use case tests for:

- Comment list succeeds for a team member.
- Comment list rejects a non-member.
- Comment list includes user data only when `CommentInclOpt::User` is requested.
- Comment list omits user data when no include option is requested.
- Comment create succeeds for a team member and persists the row.
- Comment create rejects a non-member without mutation.
- Announcement list succeeds for a team member.
- Announcement list rejects a non-member.
- Announcement list includes user data only when `AnnouncementInclOpt::User` is requested.
- Announcement list omits user data when no include option is requested.
- Announcement create succeeds for a team admin and persists the row.
- Announcement create rejects a non-admin member without mutation.
- Announcement create rejects a non-member without mutation.

Add mock repository tests for:

- Comment list filtering, sorting, pagination, and include behavior.
- Comment create duplicate rejection.
- Announcement list filtering, sorting, pagination, and include behavior.
- Announcement create duplicate rejection.

## Non-Goals

- No real Diesel repository implementation.
- No HTTP handler, router, or OpenAPI implementation.
- No legacy `domain/query/infra/api` architecture changes.
- No delete or update use cases for comments or announcements.
- No nested include graph beyond direct `User` include.
