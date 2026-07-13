//! User use cases — profile, avatar management, activity tracking, and deletion.

use std::time::Duration;

use poprako_orchestra::Nucl;
use poprako_orchestra_extra::prom::oper::{Defer, DeferBatch};
use poprako_orchestra_extra::prom::task::Task;

use poprako_util::i18n::trl;

use crate::complex::image::ImageComplex;
use crate::data::user::{
    MarkUserAvatarUploadedParams, ReserveUserAvatarParams,
    ReserveUserAvatarPayload, UpdateUserInfoParams, UserInfoVal,
};
use crate::model::user::UserToken;
use crate::part::effect::EffectDevelop;
use crate::part::effect::event::Event;
use crate::part::effect::event::user::UserActivePayload;
use crate::part::image::ImagePool;
use crate::part::prom::Prom;
use crate::part::prom::payload::{Payload, image};
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::member::{
    DeleteMember, ListMemberInfosExcluded, UpdateMember,
};
use crate::part::repo::oper::user::{
    DeleteUser, GetUserInfo, GetUserInfoExcluded, ReserveUserAvatar, UpdateUser,
};
use crate::part::repo::user::UserRepo;
use crate::result::{ExpectedVariant, RegularError, RegularResult};

#[cfg(test)]
mod tests;

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
) -> RegularResult<UserInfoVal>
where
    R: UserRepo<C>,
    I: ImagePool,
    V: EffectDevelop + Send + Sync,
{
    let user_info = repo.run(&GetUserInfo::Id { id: &id }).await?;

    // Dispatch an activity event when the user reads their own profile.
    if token.user_id == id {
        let event = Event::UserActive(UserActivePayload {
            user_id: token.user_id,
        });

        develop.develop(event).await;
    }

    UserInfoVal::from_model(image_pool, user_info).await
}

/// Updates a user's QQ ID and nickname.
///
/// Transactional flow:
///
/// 1. **Permission check:** the caller (`token.user_id`) must match the
///    target user (`params.id`). Returns `Perm` error on mismatch.
/// 2. Updates the user's own record via [`UpdateUser::Info`].
/// 3. Propagates the new nickname to all of the user's memberships via
///    [`UpdateMember::UserNickname`].
///
/// # Type Parameters
///
/// * `N: Nucl<Context = C>` — Transaction coordinator.
/// * `C` — Context anchor.
/// * `R: UserRepo<C> + MemberRepo<C>` — User and member storage.
pub async fn update_info<N, C, R>(
    nucl: &N,
    repo: &R,
    token: UserToken,
    params: UpdateUserInfoParams,
) -> RegularResult<()>
where
    N: Nucl<Context = C, Error = RegularError>,
    C: Send,
    R: UserRepo<C> + MemberRepo<C> + Send + Sync,
{
    // Only the user themselves can update their own profile.
    if token.user_id != params.id {
        return Err(RegularError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-forbidden"),
        });
    }

    nucl.coord(async move |context| -> Result<(), RegularError> {
        //

        repo.step(
            context,
            &UpdateUser::Info {
                id: &token.user_id,
                qid: &params.qid,
                nickname: &params.nickname,
            },
        )
        .await?;

        repo.step(
            context,
            &UpdateMember::UserNickname {
                user_id: &token.user_id,
                user_nickname: &params.nickname,
            },
        )
        .await?;

        Ok(())
    })
    .await?;

    Ok(())
}

/// Reserves a new avatar upload slot for a user.
///
/// Transactional flow (same pattern as [`team::reserve_avatar`]):
///
/// 1. Calls [`ReserveUserAvatar`] — generates an object key, increments
///    the version, and returns any previous avatar key.
/// 2. If replacing, defers an image-delete payload.
/// 3. Defers an image upload-check payload with a 15-minute delay.
///
/// After commit, generates a signed PUT URL.
///
/// # Type Parameters
///
/// * `N: Nucl<Context = C>` — Transaction coordinator.
/// * `C` — Context anchor.
/// * `R: UserRepo<C>` — User storage.
/// * `P: Prom<C>` — Prom enqueuer.
/// * `I: ImagePool` — Generates the signed upload URL.
///
/// [`team::reserve_avatar`]: super::team::reserve_avatar
pub async fn reserve_avatar<N, C, R, P, I>(
    nucl: &N,
    repo: &R,
    prom: &P,
    image_pool: &I,
    token: UserToken,
    params: ReserveUserAvatarParams,
) -> RegularResult<ReserveUserAvatarPayload>
where
    N: Nucl<Context = C, Error = RegularError>,
    C: Send,
    R: UserRepo<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
    I: ImagePool,
{
    let (object_key, avatar_version) = nucl
        .coord(async move |context| -> RegularResult<(String, u32)> {
            //
            let avatar_reservation = repo
                .step(
                    context,
                    &ReserveUserAvatar {
                        id: &token.user_id,
                        file_ext: &params.file_ext,
                    },
                )
                .await?;

            let mut batch_ids = Vec::new();

            let mut batch_payloads = Vec::new();

            let mut batch_delays = Vec::new();

            // If replacing an existing avatar, schedule deletion of the old object.
            if let Some(prev_key) = &avatar_reservation.prev_object_key {
                batch_ids.push(ImageComplex::gen_delete_id());

                batch_payloads.push(Payload::Image(image::Payload::Delete {
                    object_key: prev_key.clone(),
                }));

                batch_delays.push(None);
            }

            batch_ids.push(ImageComplex::gen_check_id());

            batch_payloads.push(Payload::Image(image::Payload::CheckUpload {
                resource_kind: image::ResourceKind::UserAvatar,
                resource_id: token.user_id.clone(),
                object_key: avatar_reservation.object_key.clone(),
                version: avatar_reservation.avatar_version,
            }));

            batch_delays.push(Some(Duration::from_secs(15 * 60)));

            let batch_tasks: Vec<_> = batch_ids
                .iter()
                .zip(batch_payloads.iter())
                .zip(batch_delays.iter())
                .map(|((id, payload), delay)| Task {
                    id,
                    payload,
                    delay: *delay,
                })
                .collect();

            prom.step(context, &DeferBatch::new(&batch_tasks)).await?;

            Ok((
                avatar_reservation.object_key,
                avatar_reservation.avatar_version,
            ))
        })
        .await?;

    let put_url = image_pool.get_upload_url(&object_key).await?.to_string();

    Ok(ReserveUserAvatarPayload {
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
/// * `N: Nucl<Context = C>` — Transaction coordinator.
/// * `C` — Context anchor.
/// * `R: UserRepo<C>` — User storage.
pub async fn mark_avatar_uploaded<N, C, R>(
    nucl: &N,
    repo: &R,
    token: UserToken,
    id: String,
    params: MarkUserAvatarUploadedParams,
) -> RegularResult<()>
where
    N: Nucl<Context = C, Error = RegularError>,
    C: Send,
    R: UserRepo<C> + Send + Sync,
{
    if token.user_id != id {
        return Err(RegularError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-forbidden"),
        });
    }

    nucl.coord(async move |context| -> Result<(), RegularError> {
        //

        repo.step(
            context,
            &UpdateUser::MarkAvatarUploaded {
                id: &id,
                avatar_version: params.avatar_version,
            },
        )
        .await?;

        Ok(())
    })
    .await?;

    Ok(())
}

/// Deletes a user account and all associated params.
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
/// * `N: Nucl<Context = C>` — Transaction coordinator.
/// * `C` — Context anchor.
/// * `R: UserRepo<C> + MemberRepo<C>` — User and member storage.
/// * `P: Prom<C>` — Prom enqueuer for deferred avatar deletion.
pub async fn delete<N, C, R, P>(
    nucl: &N,
    repo: &R,
    prom: &P,
    token: UserToken,
    id: String,
) -> RegularResult<()>
where
    N: Nucl<Context = C, Error = RegularError>,
    C: Send,
    R: UserRepo<C> + MemberRepo<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
{
    if token.user_id != id {
        // TODO: perm check.
        return Err(RegularError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-forbidden"),
        });
    }

    nucl.coord(async move |context| -> Result<(), RegularError> {
        //

        let user_info = repo
            .step(context, &GetUserInfoExcluded::Id { id: &id })
            .await?;

        // Delete all memberships before the user to satisfy FK constraints.

        let member_infos = repo
            .step(context, &ListMemberInfosExcluded::User { user_id: &id })
            .await?;

        for member_info in &member_infos {
            repo.step(
                context,
                &DeleteMember {
                    id: &member_info.id,
                },
            )
            .await?;
        }

        repo.step(context, &DeleteUser { id: &id }).await?;

        // Enqueue avatar object deletion if one was uploaded.
        if let Some(avatar_key) = &user_info.avatar_key
            && user_info.avatar_uploaded
        {
            let delete_id = ImageComplex::gen_delete_id();

            let payload = Payload::Image(image::Payload::Delete {
                object_key: avatar_key.clone(),
            });

            let task = Task {
                id: &delete_id,
                payload: &payload,
                delay: None,
            };

            prom.step(context, &Defer::new(task)).await?;
        }

        Ok(())
    })
    .await?;

    Ok(())
}
