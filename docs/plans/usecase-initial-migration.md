# Usecase Initial Migration Plan

This plan migrates only the initial `auth`, `user`, and `team` usecase layer to
the new transaction-step architecture.

Do not touch concrete implementation files in this slice. In particular, do not
edit `src/infra/query/**`, `src/infra/query/memory_mock/**`, `src/part_impl/**`,
`src/harness.rs`, or HTTP handlers.

## Scope

- Behavior source: current Rust legacy usecases in `src/usecase_legacy/user.rs`
  and `src/usecase_legacy/team.rs`.
- Public usecase names:
  - `auth::register`
  - `auth::login`
  - `user::*`
  - `team::*`
- Query operation naming:
  - The old `part/query/action` module must be renamed to `part/query/step`.
  - Struct names may stay domain-operation oriented (`UserGetInfoById`,
    `TeamReserveAvatar`), but the module name is `step`, not `action`.
- Local-message behavior:
  - Do not add a local-message query trait.
  - Use `part::pledge::{Append, Pledge}` for local-message/table appends.
  - Transaction compatibility is expressed by sharing the same `H` handle between
    query transactional traits and `Pledge<H>`.
- Out of scope:
  - Concrete `Execute` / `Advance` implementations.
  - API routing, OpenAPI, and handler rewiring.
  - Go-only permission behavior absent from Rust legacy.

## Naming Rules For This Slice

- Request-direction DTOs end with `Data`.
- Response/read DTOs end with `Val`.
- Do not introduce abbreviations such as `Upd`; use `Update`.
- Step files must import leaf types with `use` and then refer to leaf names.
  Do not write outputs such as `model::member::MemberInfo`.
- Source comments must stay English-only.

## Module Renames

### `src/part/query/action.rs`

Rename this file to:

```text
src/part/query/step.rs
```

Then expose:

```rust
pub mod member;
pub mod member_invitation;
pub mod team;
pub mod user;
pub mod workset;
```

Do not add `local_message`.

### `src/part/query/action/`

Rename the directory to:

```text
src/part/query/step/
```

Move existing files:

```text
src/part/query/action/user.rs   -> src/part/query/step/user.rs
src/part/query/action/member.rs -> src/part/query/step/member.rs
```

Add new files:

```text
src/part/query/step/member_invitation.rs
src/part/query/step/team.rs
src/part/query/step/workset.rs
```

### `src/part/query.rs`

Change module registration from `action` to `step`:

```rust
pub mod member;
pub mod member_invitation;
pub mod step;
pub mod team;
pub mod user;
pub mod workset;
```

Keep `Execute`, `DeriveTransactional`, and `map_drive_err`.

## Data And Model Files

### `src/data.rs`

Register team data:

```rust
pub mod auth;
pub mod team;
pub mod user;
```

### `src/data/auth.rs`

Replace the current login-only structs with:

```rust
pub struct RegisterData {
    pub qid: String,
    pub nickname: String,
    pub password: String,
    pub invitation_code: String,
}

pub struct RegisterVal {
    pub user_id: String,
    pub token: String,
}

pub struct LoginData {
    pub qid: String,
    pub password: String,
}

pub struct LoginVal {
    pub user_id: String,
    pub token: String,
}
```

### `src/data/user.rs`

Use these names and shapes:

```rust
pub struct UserInfoVal {
    pub id: String,
    pub nickname: String,
    pub qid: String,
    pub avatar_url: Option<String>,
    pub is_sadmin: bool,
    pub last_active_at: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

pub struct UserInfoUpdateData {
    pub id: String,
    pub qid: String,
    pub nickname: String,
}

pub struct UserAvatarReserveData {
    pub file_extension: String,
}

pub struct UserAvatarReserveVal {
    pub put_url: String,
    pub avatar_version: i64,
}

pub struct UserAvatarMarkUploadedData {
    pub avatar_version: i64,
}
```

`UserInfoVal::from_model`:

- Accepts `&impl ImagePool` and `UserInfo`.
- Uses `get_signed` only when `avatar_uploaded` is true and `avatar_key` is
  present.
- Converts signed-url failures to `None`, matching legacy behavior.
- Converts timestamps with `ToUnixMilli`.

### `src/data/team.rs`

Create this file:

```rust
pub struct TeamInfoVal {
    pub id: String,
    pub name: String,
    pub description: String,
    pub avatar_url: Option<String>,
    pub workset_next_index: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

pub struct TeamCreateData {
    pub name: String,
    pub description: String,
}

pub struct TeamInfoUpdateData {
    pub name: String,
    pub description: String,
}

pub struct TeamAvatarReserveData {
    pub file_extension: String,
}

pub struct TeamAvatarReserveVal {
    pub put_url: String,
    pub avatar_version: i64,
}

pub struct TeamAvatarMarkUploadedData {
    pub avatar_version: i64,
}
```

`TeamInfoVal::from_model` mirrors `UserInfoVal::from_model`.

### `src/model.rs`

Register the required model modules:

```rust
pub mod local_message;
pub mod member;
pub mod member_invitation;
pub mod team;
pub mod user;
pub mod workset;
```

### `src/model/user.rs`

Keep existing `UserToken`, `UserInfo`, and `UserForm`. Rename any abbreviated
profile-update model to the full name below:

```rust
pub struct UserInfoUpdate<'a> {
    pub id: &'a str,
    pub qid: &'a str,
    pub nickname: &'a str,
}
```

Add:

```rust
pub struct UserAvatarReservation {
    pub object_key: String,
    pub previous_object_key: Option<String>,
    pub avatar_version: i64,
}

pub struct UserCredential {
    pub user_id: String,
    pub password_hash: String,
}
```

`UserCredential::verify_password` delegates to `atom::auth::verify_password`.

### `src/model/member.rs`

Ensure this file has the minimum types needed by usecases:

```rust
use crate::domain::model::value::role::RoleMask;

pub struct MemberInfo {
    pub id: String,
    pub user_id: String,
    pub user_nickname: String,
    pub team_id: String,
}

pub struct MemberForm {
    pub id: String,
    pub user_id: String,
    pub user_nickname: String,
    pub team_id: String,
    pub role_mask: RoleMask,
}
```

### `src/model/member_invitation.rs`

Create:

```rust
use crate::domain::model::value::role::RoleMask;

pub struct MemberInvitationInfo {
    pub id: String,
    pub team_id: String,
    pub invitor_id: String,
    pub invitee_qid: String,
    pub role_mask: RoleMask,
}
```

### `src/model/local_message.rs`

Create:

```rust
pub use crate::domain::model::value::local_message::{
    ImageLocalMessage,
    ImageResourceKind,
    IMAGE_TOPIC,
};
```

This is a value-module bridge only. It is not a query abstraction.

### `src/model/team.rs`

Create:

```rust
use time::OffsetDateTime;

pub struct TeamInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub avatar_key: Option<String>,
    pub avatar_uploaded: bool,
    pub avatar_version: i64,
    pub workset_next_index: i32,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

pub struct TeamForm {
    pub id: String,
    pub name: String,
    pub description: String,
}

pub struct TeamInfoUpdate<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub description: &'a str,
}

pub struct TeamAvatarReservation {
    pub object_key: String,
    pub previous_object_key: Option<String>,
    pub avatar_version: i64,
}
```

### `src/model/workset.rs`

Create the minimal model needed for team delete cascade:

```rust
pub struct WorksetInfo {
    pub id: String,
}
```

## Step Files

### `src/part/query/step/user.rs`

Use pure imports:

```rust
use poprako_transactional::step::Step;

use crate::model::user::{
    UserAvatarReservation,
    UserCredential,
    UserForm,
    UserInfo,
    UserInfoUpdate,
};
```

Define:

```rust
pub struct UserGetInfoById<'a> {
    pub id: &'a str,
}

impl<'a> Step for UserGetInfoById<'a> {
    type Output = UserInfo;
}

pub struct UserGetCredentialByQid<'a> {
    pub qid: &'a str,
}

impl<'a> Step for UserGetCredentialByQid<'a> {
    type Output = UserCredential;
}

pub struct UserCreate<'a> {
    pub form: &'a UserForm,
}

impl<'a> Step for UserCreate<'a> {
    type Output = UserInfo;
}

pub struct UserUpdateInfo<'a> {
    pub input: UserInfoUpdate<'a>,
}

impl<'a> Step for UserUpdateInfo<'a> {
    type Output = ();
}

pub struct UserReserveAvatar<'a> {
    pub id: &'a str,
    pub file_extension: &'a str,
}

impl<'a> Step for UserReserveAvatar<'a> {
    type Output = UserAvatarReservation;
}

pub struct UserMarkAvatarUploaded<'a> {
    pub id: &'a str,
    pub avatar_version: i64,
}

impl<'a> Step for UserMarkAvatarUploaded<'a> {
    type Output = ();
}

pub struct UserTouchLastActive<'a> {
    pub id: &'a str,
}

impl<'a> Step for UserTouchLastActive<'a> {
    type Output = ();
}

pub struct UserGetInfoExcluded<'a> {
    pub id: &'a str,
}

impl<'a> Step for UserGetInfoExcluded<'a> {
    type Output = UserInfo;
}

pub struct UserDelete<'a> {
    pub id: &'a str,
}

impl<'a> Step for UserDelete<'a> {
    type Output = ();
}
```

### `src/part/query/step/member.rs`

Use pure imports:

```rust
use poprako_transactional::step::Step;

use crate::model::member::{MemberForm, MemberInfo};
```

Define:

```rust
pub struct MemberCreate<'a> {
    pub form: &'a MemberForm,
}

impl<'a> Step for MemberCreate<'a> {
    type Output = MemberInfo;
}

pub struct MemberUpdateUserNickname<'a> {
    pub user_id: &'a str,
    pub user_nickname: &'a str,
}

impl<'a> Step for MemberUpdateUserNickname<'a> {
    type Output = ();
}

pub struct MemberTouchLastActive<'a> {
    pub user_id: &'a str,
}

impl<'a> Step for MemberTouchLastActive<'a> {
    type Output = ();
}

pub struct MemberListByUserIdExcluded<'a> {
    pub user_id: &'a str,
}

impl<'a> Step for MemberListByUserIdExcluded<'a> {
    type Output = Vec<MemberInfo>;
}

pub struct MemberDelete<'a> {
    pub id: &'a str,
}

impl<'a> Step for MemberDelete<'a> {
    type Output = ();
}
```

### `src/part/query/step/member_invitation.rs`

Use pure imports:

```rust
use poprako_transactional::step::Step;

use crate::model::member_invitation::MemberInvitationInfo;
```

Define:

```rust
pub struct MemberInvitationGetByCodeExcluded<'a> {
    pub invitation_code: &'a str,
}

impl<'a> Step for MemberInvitationGetByCodeExcluded<'a> {
    type Output = MemberInvitationInfo;
}

pub struct MemberInvitationMarkPendingAsUsed<'a> {
    pub id: &'a str,
}

impl<'a> Step for MemberInvitationMarkPendingAsUsed<'a> {
    type Output = ();
}
```

### `src/part/query/step/team.rs`

Use pure imports:

```rust
use poprako_transactional::step::Step;
use poprako_util::page::Page;

use crate::model::team::{
    TeamAvatarReservation,
    TeamForm,
    TeamInfo,
    TeamInfoUpdate,
};
```

Define:

```rust
pub struct TeamCreate<'a> {
    pub form: &'a TeamForm,
}

impl<'a> Step for TeamCreate<'a> {
    type Output = TeamInfo;
}

pub struct TeamGetInfoById<'a> {
    pub id: &'a str,
}

impl<'a> Step for TeamGetInfoById<'a> {
    type Output = TeamInfo;
}

pub struct TeamList {
    pub page: Page,
}

impl Step for TeamList {
    type Output = Vec<TeamInfo>;
}

pub struct TeamUpdateInfo<'a> {
    pub input: TeamInfoUpdate<'a>,
}

impl<'a> Step for TeamUpdateInfo<'a> {
    type Output = ();
}

pub struct TeamReserveAvatar<'a> {
    pub id: &'a str,
    pub file_extension: &'a str,
}

impl<'a> Step for TeamReserveAvatar<'a> {
    type Output = TeamAvatarReservation;
}

pub struct TeamMarkAvatarUploaded<'a> {
    pub id: &'a str,
    pub avatar_version: i64,
}

impl<'a> Step for TeamMarkAvatarUploaded<'a> {
    type Output = ();
}

pub struct TeamGetInfoExcluded<'a> {
    pub id: &'a str,
}

impl<'a> Step for TeamGetInfoExcluded<'a> {
    type Output = TeamInfo;
}

pub struct TeamDelete<'a> {
    pub id: &'a str,
}

impl<'a> Step for TeamDelete<'a> {
    type Output = ();
}
```

### `src/part/query/step/workset.rs`

Use pure imports:

```rust
use poprako_transactional::step::Step;

use crate::model::workset::WorksetInfo;
```

Define:

```rust
pub struct WorksetListByTeamIdExcluded<'a> {
    pub team_id: &'a str,
}

impl<'a> Step for WorksetListByTeamIdExcluded<'a> {
    type Output = Vec<WorksetInfo>;
}

pub struct WorksetDeleteCascade<'a> {
    pub id: &'a str,
}

impl<'a> Step for WorksetDeleteCascade<'a> {
    type Output = ();
}
```

## Query Trait Files

### `src/part/query/user.rs`

Update imports to `step`, not `action`:

```rust
use poprako_transactional::advance::Advance;

use crate::part::query::step::user::{
    UserCreate,
    UserDelete,
    UserGetCredentialByQid,
    UserGetInfoById,
    UserGetInfoExcluded,
    UserMarkAvatarUploaded,
    UserReserveAvatar,
    UserTouchLastActive,
    UserUpdateInfo,
};
use crate::part::query::{DeriveTransactional, Execute};
use crate::result::RootError;
```

Trait shape:

```rust
pub trait UserQuery<H>:
    DeriveTransactional
    + for<'a> Execute<UserGetInfoById<'a>, Error = RootError>
    + for<'a> Execute<UserGetCredentialByQid<'a>, Error = RootError>
where
    Self::Transactional: UserQueryTransactional<H>,
{
}

pub trait UserQueryTransactional<H>:
    for<'a> Advance<UserCreate<'a>, H, Error = RootError>
    + for<'a> Advance<UserUpdateInfo<'a>, H, Error = RootError>
    + for<'a> Advance<UserReserveAvatar<'a>, H, Error = RootError>
    + for<'a> Advance<UserMarkAvatarUploaded<'a>, H, Error = RootError>
    + for<'a> Advance<UserTouchLastActive<'a>, H, Error = RootError>
    + for<'a> Advance<UserGetInfoExcluded<'a>, H, Error = RootError>
    + for<'a> Advance<UserDelete<'a>, H, Error = RootError>
{
}
```

### `src/part/query/member.rs`

Update imports and names:

```rust
use poprako_transactional::advance::Advance;

use crate::part::query::step::member::{
    MemberCreate,
    MemberDelete,
    MemberListByUserIdExcluded,
    MemberTouchLastActive,
    MemberUpdateUserNickname,
};
use crate::part::query::DeriveTransactional;
use crate::result::RootError;
```

Trait shape:

```rust
pub trait MemberQuery<H>: DeriveTransactional
where
    Self::Transactional: MemberQueryTransactional<H>,
{
}

pub trait MemberQueryTransactional<H>:
    for<'a> Advance<MemberCreate<'a>, H, Error = RootError>
    + for<'a> Advance<MemberUpdateUserNickname<'a>, H, Error = RootError>
    + for<'a> Advance<MemberTouchLastActive<'a>, H, Error = RootError>
    + for<'a> Advance<MemberListByUserIdExcluded<'a>, H, Error = RootError>
    + for<'a> Advance<MemberDelete<'a>, H, Error = RootError>
    + Sized
{
}
```

### `src/part/query/member_invitation.rs`

Create:

```rust
use poprako_transactional::advance::Advance;

use crate::part::query::step::member_invitation::{
    MemberInvitationGetByCodeExcluded,
    MemberInvitationMarkPendingAsUsed,
};
use crate::part::query::DeriveTransactional;
use crate::result::RootError;

pub trait MemberInvitationQuery<H>: DeriveTransactional
where
    Self::Transactional: MemberInvitationQueryTransactional<H>,
{
}

pub trait MemberInvitationQueryTransactional<H>:
    for<'a> Advance<MemberInvitationGetByCodeExcluded<'a>, H, Error = RootError>
    + for<'a> Advance<MemberInvitationMarkPendingAsUsed<'a>, H, Error = RootError>
{
}
```

### `src/part/query/team.rs`

Create:

```rust
use poprako_transactional::advance::Advance;

use crate::part::query::step::team::{
    TeamCreate,
    TeamDelete,
    TeamGetInfoById,
    TeamGetInfoExcluded,
    TeamList,
    TeamMarkAvatarUploaded,
    TeamReserveAvatar,
    TeamUpdateInfo,
};
use crate::part::query::{DeriveTransactional, Execute};
use crate::result::RootError;

pub trait TeamQuery<H>:
    DeriveTransactional
    + for<'a> Execute<TeamCreate<'a>, Error = RootError>
    + for<'a> Execute<TeamGetInfoById<'a>, Error = RootError>
    + Execute<TeamList, Error = RootError>
    + for<'a> Execute<TeamUpdateInfo<'a>, Error = RootError>
    + for<'a> Execute<TeamMarkAvatarUploaded<'a>, Error = RootError>
where
    Self::Transactional: TeamQueryTransactional<H>,
{
}

pub trait TeamQueryTransactional<H>:
    for<'a> Advance<TeamReserveAvatar<'a>, H, Error = RootError>
    + for<'a> Advance<TeamMarkAvatarUploaded<'a>, H, Error = RootError>
    + for<'a> Advance<TeamGetInfoExcluded<'a>, H, Error = RootError>
    + for<'a> Advance<TeamDelete<'a>, H, Error = RootError>
{
}
```

### `src/part/query/workset.rs`

Create:

```rust
use poprako_transactional::advance::Advance;

use crate::part::query::step::workset::{
    WorksetDeleteCascade,
    WorksetListByTeamIdExcluded,
};
use crate::part::query::DeriveTransactional;
use crate::result::RootError;

pub trait WorksetQuery<H>: DeriveTransactional
where
    Self::Transactional: WorksetQueryTransactional<H>,
{
}

pub trait WorksetQueryTransactional<H>:
    for<'a> Advance<WorksetListByTeamIdExcluded<'a>, H, Error = RootError>
    + for<'a> Advance<WorksetDeleteCascade<'a>, H, Error = RootError>
{
}
```

## Pledge Changes

### `src/part/pledge.rs`

The existing file already defines `Append<'a>` and `Pledge<H>`. Extend
`Payload` so usecases can append image-related local messages without using a
local-message query:

```rust
use crate::model::local_message::ImageLocalMessage;

#[derive(Serialize, Deserialize)]
pub enum Payload {
    Image(ImageLocalMessage),
}
```

Do not add any local-message query abstraction in the new `part/query` tree.

Usecase code appends image messages by advancing `Append` against a transaction
handle through a dependency whose transactional type implements `Pledge<H>`.

## Other Support Files

### `src/part/image_pool.rs`

Extend the trait:

```rust
#[async_trait]
pub trait ImagePool {
    async fn get_signed(&self, key: &str) -> RootResult<Url>;

    async fn put_signed(&self, key: &str) -> RootResult<Url>;
}
```

Do not implement this trait in this slice.

### `src/atom/auth.rs`

Replace both `todo!()` functions:

```rust
pub fn hash_password(password: &str) -> RootResult<String>;

pub fn verify_password(password: &str, password_hash: &str) -> bool;
```

Behavior:

- `hash_password` uses bcrypt with `bcrypt::DEFAULT_COST`.
- bcrypt errors map to `RootError::Unrecoverable` with a
  `[auth::hash_password]` prefix.
- `verify_password` returns `false` on verification errors.

### `src/part/token.rs`

Create and register from `src/part.rs`:

```rust
use crate::model::user::UserToken;
use crate::result::RootResult;

pub trait TokenIssuer {
    fn sign(&self, token: &UserToken) -> RootResult<String>;
}
```

## Usecase Files

### `src/usecase.rs`

Register:

```rust
pub mod auth;
pub mod team;
pub mod user;
```

### `src/usecase/auth.rs`

Implement `register`:

```rust
pub async fn register<D, H, Q, T, E>(
    drive: D,
    query: Q,
    token_issuer: &T,
    develop: &E,
    input: RegisterData,
) -> RootResult<RegisterVal>
```

Bounds:

- `D: Drive<H>`
- `D::Error: Into<RootError>`
- `H: Send`
- `Q: UserQuery<H> + MemberQuery<H> + MemberInvitationQuery<H> + Send`
- `<Q as DeriveTransactional>::Transactional:
  UserQueryTransactional<H>
  + MemberQueryTransactional<H>
  + MemberInvitationQueryTransactional<H>
  + Send`
- `T: TokenIssuer`
- `E: Develop + Send + Sync`

Flow:

1. Run `drive.run_transactional`.
2. Inside the transaction, derive `let mut query =
   DeriveTransactional::transactional(&query).await`.
3. Advance `MemberInvitationGetByCodeExcluded`.
4. Reject qid mismatch with `ExpectedVariant::Args` and
   `trl("error-invalid-invitation-code")`.
5. Hash the password.
6. Build `UserForm`.
7. Advance `UserCreate`.
8. Build `MemberForm` from created user and invitation data.
9. Advance `MemberCreate`.
10. Advance `MemberInvitationMarkPendingAsUsed`.
11. Return the created user id and post-commit signup event data from the
    transaction closure.
12. After commit, emit `Event::UserSignedUp`.
13. Sign `UserToken`.
14. Return `RegisterVal`.

Implement `login`:

```rust
pub async fn login<H, Q, T>(
    query: Q,
    token_issuer: &T,
    input: LoginData,
) -> RootResult<LoginVal>
```

Bounds:

- `Q: UserQuery<H>`
- `<Q as DeriveTransactional>::Transactional: UserQueryTransactional<H>`
- `T: TokenIssuer`

Flow:

1. Execute `UserGetCredentialByQid`.
2. Reject failed password verification with `ExpectedVariant::Auth` and
   `trl("error-wrong-credentials")`.
3. Sign `UserToken`.
4. Return `LoginVal`.

### `src/usecase/user.rs`

Imports must use `step`, not `action`.

`get_info`:

- Execute `UserGetInfoById`.
- If `token.user_id == id`, emit `Event::UserActive`.
- Return `UserInfoVal`.

`update_info`:

- Accept `UserInfoUpdateData`.
- Reject non-owner with `ExpectedVariant::Perm`.
- Transactionally advance `UserUpdateInfo`.
- Then advance `MemberUpdateUserNickname`.
- Do not use `Upd` in any new type name.

`reserve_avatar`:

```rust
pub async fn reserve_avatar<D, H, Q, P>(
    drive: D,
    query: Q,
    image_pool: P,
    token: UserToken,
    user_id: String,
    input: UserAvatarReserveData,
) -> RootResult<UserAvatarReserveVal>
```

Bounds:

- `D: Drive<H>`
- `D::Error: Into<RootError>`
- `H: Send`
- `Q: UserQuery<H> + Send`
- `<Q as DeriveTransactional>::Transactional:
  UserQueryTransactional<H> + Pledge<H> + Send`
- `P: ImagePool`

Flow:

1. Reject if `token.user_id != user_id`.
2. Transactionally advance `UserReserveAvatar`.
3. If `previous_object_key` is present, advance `Append` with
   `Payload::Image(ImageLocalMessage::delete(...))` and zero delay.
4. Advance `Append` with
   `Payload::Image(ImageLocalMessage::check_uploaded(...))` and 15-minute delay.
5. After commit, call `image_pool.put_signed`.
6. Return `UserAvatarReserveVal`.

`mark_avatar_uploaded`:

- Reject if `token.user_id != user_id`.
- Transactionally advance `UserMarkAvatarUploaded`.

`touch_last_active`:

- Transactionally advance `UserTouchLastActive`.
- Then advance `MemberTouchLastActive`.

`delete_user`:

Bounds must include:

- `<Q as DeriveTransactional>::Transactional:
  UserQueryTransactional<H> + MemberQueryTransactional<H> + Pledge<H> + Send`

Flow:

1. Advance `UserGetInfoExcluded` to capture avatar key.
2. Advance `MemberListByUserIdExcluded`.
3. Advance `MemberDelete` for every returned member.
4. Advance `UserDelete`.
5. If an avatar key existed, advance `Append` with an image-delete payload and
   zero delay.

### `src/usecase/team.rs`

Create this file.

`create`:

- Build `TeamForm`.
- Execute `TeamCreate`.
- Return `TeamInfoVal`.

`get_info`:

- Execute `TeamGetInfoById`.
- Return `TeamInfoVal`.

`list_infos`:

- Execute `TeamList`.
- Convert all items to `TeamInfoVal`.

`update_info`:

- Accept `TeamInfoUpdateData`.
- Execute `TeamUpdateInfo`.

`reserve_avatar`:

Bounds:

- `D: Drive<H>`
- `D::Error: Into<RootError>`
- `H: Send`
- `Q: TeamQuery<H> + Send`
- `<Q as DeriveTransactional>::Transactional:
  TeamQueryTransactional<H> + Pledge<H> + Send`
- `P: ImagePool`

Flow:

1. Transactionally advance `TeamReserveAvatar`.
2. Append previous-avatar delete payload through `Pledge<H>` when needed.
3. Append check-uploaded payload through `Pledge<H>` with 15-minute delay.
4. After commit, call `image_pool.put_signed`.
5. Return `TeamAvatarReserveVal`.

`mark_avatar_uploaded`:

- Execute `TeamMarkAvatarUploaded`.

`delete`:

Bounds:

- `D: Drive<H>`
- `D::Error: Into<RootError>`
- `H: Send`
- `Q: TeamQuery<H> + WorksetQuery<H> + Send`
- `<Q as DeriveTransactional>::Transactional:
  TeamQueryTransactional<H> + WorksetQueryTransactional<H> + Pledge<H> + Send`

Flow:

1. Advance `TeamGetInfoExcluded` to capture avatar key.
2. Advance `WorksetListByTeamIdExcluded`.
3. Advance `WorksetDeleteCascade` for every returned workset.
4. Advance `TeamDelete`.
5. If an avatar key existed, append an image-delete payload through `Pledge<H>`.

## Error Mapping

- `ExpectedVariant::Args`: invalid input, missing resource, stale avatar version.
- `ExpectedVariant::Auth`: invalid login credentials.
- `ExpectedVariant::Perm`: current user attempting another user's profile or
  avatar operation.
- `RootError::Unrecoverable`: bcrypt/token/internal failures.
- All user-facing messages use `trl("...")`.

Required keys if not already present:

- `error-invalid-invitation-code`
- `error-wrong-credentials`
- `error-forbidden`
- `error-stale-avatar-upload`

## Acceptance Criteria

- No new `part/query/local_message.rs`.
- No local-message query abstraction appears in new usecase or trait plans.
- `part/query/action` is fully renamed to `part/query/step`.
- Step snippets use pure `use` imports and leaf type names.
- No new `Upd` identifiers are introduced.
- User/team avatar and delete local-message effects go through `Pledge<H>` with
  the same transaction handle `H`.
- `cargo check` is expected to pass once only the planned usecase, data, model,
  step, trait, pledge, image-pool, auth-atom, and token files are edited.
