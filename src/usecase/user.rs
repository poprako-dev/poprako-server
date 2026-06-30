//! User use cases — profile, avatar management, activity tracking, and deletion.

use time::{Duration, OffsetDateTime};

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;
use poprako_util::i18n::trl;

use crate::complex::image::ImageComplex;
use crate::data::user::{
    MarkUserAvatarUploadedData, ReserveUserAvatarData, ReserveUserAvatarVal, UpdateUserInfoData,
    UserInfoVal,
};
use crate::model::user::UserToken;
use crate::part::effect::event::Event;
use crate::part::effect::event::user::UserActivePayload;
use crate::part::effect::{EffectDevelop, EffectEmit as _};
use crate::part::image::ImagePool;
use crate::part::prom::intention::{IMAGE_TOPIC, ImageIntention, ImageKind};
use crate::part::prom::{Payload, Prom, PromStep, PromTransactional};
use crate::part::repo::map_drive_err;
use crate::part::repo::member::{MemberRepo, MemberRepoTransactional};
use crate::part::repo::step::member::MemberStep;
use crate::part::repo::step::user::UserStep;
use crate::part::repo::user::{UserRepo, UserRepoTransactional};
use crate::result::{ExpectedVariant, RootError, RootResult, accept};
use crate::util::DeriveTransactional;

#[cfg(test)]
pub(crate) mod tests;

/// Fetches a user's profile with avatar URL resolution.
///
/// Non-transactional read. When the requesting user (identified by `token`)
/// reads their own profile, a [`UserActive`] event is emitted for activity
/// tracking. Other users' profiles are returned without emitting an event.
///
/// # Type Parameters
///
/// * `C` — Context anchor.
/// * `R: UserRepo<C>` — User storage.
/// * `I: ImagePool` — Resolves the avatar signed URL.
/// * `V: EffectDevelop` — Processes the activity event (only for self-reads).
pub async fn get_info<C, R, I, V>(
    repo: &R,
    image_pool: &I,
    develop: &V,
    token: UserToken,
    id: String,
) -> RootResult<UserInfoVal>
where
    R: UserRepo<C>,
    <R as DeriveTransactional>::Transactional: UserRepoTransactional<C>,
    I: ImagePool,
    V: EffectDevelop + Send + Sync,
{
    let user_info = repo.execute(&UserStep::get_info_by_id(&id)).await?;

    // Emit an activity event when the user reads their own profile.
    if token.user_id == id {
        Event::UserActive(UserActivePayload {
            user_id: token.user_id,
        })
        .emit(develop)
        .await;
    }

    UserInfoVal::from_model(image_pool, user_info).await
}

/// Updates a user's QQ ID and nickname.
///
/// Transactional flow:
///
/// 1. **Permission check:** the caller (`token.user_id`) must match the
///    target user (`data.id`). Returns `Perm` error on mismatch.
/// 2. Updates the user's own record via [`UserStep::update_info`].
/// 3. Propagates the new nickname to all of the user's memberships via
///    [`MemberStep::update_user_nickname`].
///
/// # Type Parameters
///
/// * `D: Drive<C>` — Transaction driver.
/// * `C` — Context anchor.
/// * `R: UserRepo<C> + MemberRepo<C>` — User and member storage.
pub async fn update_info<D, C, R>(
    drive: &D,
    repo: &R,
    token: UserToken,
    data: UpdateUserInfoData,
) -> RootResult<()>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: UserRepo<C> + MemberRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional:
        UserRepoTransactional<C> + MemberRepoTransactional<C> + Send,
{
    // Only the user themselves can update their own profile.
    if token.user_id != data.id {
        return Err(RootError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-forbidden"),
        });
    }

    drive
        .with_context(async move |context| {
            let repo = repo.transactional().await;

            repo.advance(
                context,
                &UserStep::update_info(&token.user_id, &data.qid, &data.nickname),
            )
            .await?;

            repo.advance(
                context,
                &MemberStep::update_user_nickname(&token.user_id, &data.nickname),
            )
            .await?;

            accept(())
        })
        .await
        .map_err(map_drive_err)?;

    accept(())
}

/// Reserves a new avatar upload slot for a user.
///
/// Transactional flow (same pattern as [`team::reserve_avatar`]):
///
/// 1. Calls [`UserStep::reserve_avatar`] — generates an object key, increments
///    the version, and returns any previous avatar key.
/// 2. If replacing, enqueues a [`Delete`](ImageIntention::Delete) prom record.
/// 3. Enqueues a [`CheckUploaded`](ImageIntention::CheckUploaded) prom record
///    (visible after 15 minutes).
///
/// After commit, generates a signed PUT URL.
///
/// # Type Parameters
///
/// * `D: Drive<C>` — Transaction driver.
/// * `C` — Context anchor.
/// * `R: UserRepo<C>` — User storage.
/// * `P: Prom<C>` — Prom enqueuer.
/// * `I: ImagePool` — Generates the signed upload URL.
///
/// [`team::reserve_avatar`]: super::team::reserve_avatar
pub async fn reserve_avatar<D, C, R, P, I>(
    drive: &D,
    repo: &R,
    prom: &P,
    image_pool: &I,
    token: UserToken,
    data: ReserveUserAvatarData,
) -> RootResult<ReserveUserAvatarVal>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: UserRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: UserRepoTransactional<C> + Send,
    P: Prom<C> + Send + Sync,
    <P as DeriveTransactional>::Transactional: PromTransactional<C> + Send + Sync,
    I: ImagePool,
{
    let (object_key, avatar_version) = drive
        .with_context(async move |context| {
            let repo = repo.transactional().await;
            let prom = prom.transactional().await;

            let avatar_reservation = repo
                .advance(
                    context,
                    &UserStep::reserve_avatar(&token.user_id, &data.file_ext),
                )
                .await?;

            let now = OffsetDateTime::now_utc();

            // If replacing an existing avatar, schedule deletion of the old object.
            if let Some(prev_key) = &avatar_reservation.prev_object_key {
                let delete_id = ImageComplex::gen_delete_id();

                prom.advance(
                    context,
                    &PromStep::append(
                        &delete_id,
                        IMAGE_TOPIC,
                        Payload::Image(ImageIntention::Delete {
                            object_key: prev_key.clone(),
                        }),
                        &now,
                    ),
                )
                .await?;
            }

            let check_id = ImageComplex::gen_check_id();
            let check_visible_at = now + Duration::minutes(15);

            prom.advance(
                context,
                &PromStep::append(
                    &check_id,
                    IMAGE_TOPIC,
                    Payload::Image(ImageIntention::CheckUploaded {
                        kind: ImageKind::UserAvatar,
                        resource_id: token.user_id.clone(),
                        object_key: avatar_reservation.object_key.clone(),
                        image_version: avatar_reservation.avatar_version,
                    }),
                    &check_visible_at,
                ),
            )
            .await?;

            accept((
                avatar_reservation.object_key,
                avatar_reservation.avatar_version,
            ))
        })
        .await
        .map_err(map_drive_err)?;

    let put_url = image_pool.put_signed(&object_key).await?.to_string();

    Ok(ReserveUserAvatarVal {
        put_url,
        avatar_version,
    })
}

/// Marks a reserved user avatar as successfully uploaded.
///
/// Transactional — the update runs inside a short-lived transaction so the
/// version check and mark are atomic. The caller must own the resource
/// (`token.user_id == id`).
///
/// # Type Parameters
///
/// * `D: Drive<C>` — Transaction driver.
/// * `C` — Context anchor.
/// * `R: UserRepo<C>` — User storage.
pub async fn mark_avatar_uploaded<D, C, R>(
    drive: &D,
    repo: &R,
    token: UserToken,
    id: String,
    data: MarkUserAvatarUploadedData,
) -> RootResult<()>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: UserRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: UserRepoTransactional<C> + Send,
{
    if token.user_id != id {
        return Err(RootError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-forbidden"),
        });
    }

    drive
        .with_context(async move |context| {
            let repo = repo.transactional().await;

            repo.advance(
                context,
                &UserStep::mark_avatar_uploaded(&id, data.avatar_version),
            )
            .await?;

            accept(())
        })
        .await
        .map_err(map_drive_err)?;

    accept(())
}

/// Updates the `last_active_at` timestamp on both the user record and all
/// associated memberships.
///
/// Transactional — the user and member updates are atomic.
///
/// # Type Parameters
///
/// * `D: Drive<C>` — Transaction driver.
/// * `C` — Context anchor.
/// * `R: UserRepo<C> + MemberRepo<C>` — User and member storage.
pub async fn touch_last_active<D, C, R>(drive: &D, repo: &R, token: UserToken) -> RootResult<()>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: UserRepo<C> + MemberRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional:
        UserRepoTransactional<C> + MemberRepoTransactional<C> + Send,
{
    drive
        .with_context(async move |context| {
            let repo = repo.transactional().await;

            repo.advance(context, &UserStep::touch_last_active(&token.user_id))
                .await?;

            repo.advance(context, &MemberStep::touch_last_active(&token.user_id))
                .await?;

            accept(())
        })
        .await
        .map_err(map_drive_err)?;

    accept(())
}

/// Deletes a user account and all associated data.
///
/// Transactional cascade:
///
/// 1. **Permission check:** the caller must own the account. Returns `Perm`
///    error on mismatch.
/// 2. Fetches the user info with a pessimistic lock.
/// 3. Lists and deletes all of the user's memberships (must happen before
///    the user row is deleted due to foreign key constraints).
/// 4. Deletes the user itself.
/// 5. If the user had an uploaded avatar, enqueues a prom record to delete
///    the avatar object from storage.
///
/// # Type Parameters
///
/// * `D: Drive<C>` — Transaction driver.
/// * `C` — Context anchor.
/// * `R: UserRepo<C> + MemberRepo<C>` — User and member storage.
/// * `P: Prom<C>` — Prom enqueuer for deferred avatar deletion.
pub async fn delete<D, C, R, P>(
    drive: &D,
    repo: &R,
    prom: &P,
    token: UserToken,
    id: String,
) -> RootResult<()>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: UserRepo<C> + MemberRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional:
        UserRepoTransactional<C> + MemberRepoTransactional<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
    <P as DeriveTransactional>::Transactional: PromTransactional<C> + Send + Sync,
{
    if token.user_id != id {
        // TODO: perm check.
        return Err(RootError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-forbidden"),
        });
    }

    drive
        .with_context(async move |context| {
            let repo = repo.transactional().await;
            let prom = prom.transactional().await;

            let user_info = repo
                .advance(context, &UserStep::get_info_excluded(&id))
                .await?;

            // Delete all memberships before the user to satisfy FK constraints.
            let member_infos = repo
                .advance(context, &MemberStep::list_infos_by_user_id_excluded(&id))
                .await?;

            for mi in &member_infos {
                repo.advance(context, &MemberStep::delete(&mi.id)).await?;
            }

            repo.advance(context, &UserStep::delete(&id)).await?;

            // Enqueue avatar object deletion if one was uploaded.
            if let Some(avatar_key) = &user_info.avatar_key
                && user_info.avatar_uploaded
            {
                let now = OffsetDateTime::now_utc();
                let delete_id = ImageComplex::gen_delete_id();

                prom.advance(
                    context,
                    &PromStep::append(
                        &delete_id,
                        IMAGE_TOPIC,
                        Payload::Image(ImageIntention::Delete {
                            object_key: avatar_key.clone(),
                        }),
                        &now,
                    ),
                )
                .await?;
            }

            accept(())
        })
        .await
        .map_err(map_drive_err)?;

    accept(())
}
