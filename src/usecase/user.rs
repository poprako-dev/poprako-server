//! User use cases — profile, avatar management, activity tracking, and deletion.

use std::time::Duration;

use poprako_orchestra::{Nucl, OperRun as _, OperStep as _};
use poprako_orchestra_extra::prom::oper::{Defer, DeferBatch};
use poprako_orchestra_extra::prom::task::Task;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::image::ImageComplex;
use crate::complex::user::UserComplex;
use crate::data::instr::user::{
    MarkUserAvatarUploadedInstr, ReserveUserAvatarInstr, UpdateUserInfoInstr,
    UpdateUserPasswordInstr,
};
use crate::data::val::user::ReserveUserAvatarVal;
use crate::data::view::image::ImageUploadSlotView;
use crate::data::view::user::UserInfoView;
use crate::model::shared::user::UserToken;
use crate::model::write::member::MemberNicknameRepl;
use crate::model::write::user::{UserAvatarRepl, UserCredsRepl, UserInfoRepl};
use crate::part::effect::event::Event;
use crate::part::effect::event::user::UserActiveEvent;
use crate::part::effect::{Develop, EffectEvent as _};
use crate::part::image::{ImageManager, ImagePool, ImageUploadSpec};
use crate::part::prom::Prom;
use crate::part::prom::payload::{TaskPayload, image};
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::member::{
    DeleteMember, ListMemberInfosExcluded, UpdateMember,
};
use crate::part::repo::oper::user::{
    DeleteUser, GetUserCredential, GetUserInfo, GetUserInfoExcluded,
    ReserveUserAvatar, UpdateUser,
};
use crate::part::repo::user::UserRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

pub use delete::delete;

// User deletion use case.
mod delete;

#[cfg(test)]
// Unit tests for account, role, and membership operations.
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
/// * `D: EffectDevelop` — Processes the activity event (only for self-reads).
#[instrument(level = "info", skip(repo, image_pool, develop))]
pub async fn get_info<C, R, I, D>(
    (repo, image_pool, develop): (&R, &I, &D),
    token: UserToken,
    id: String,
) -> BaseRest<UserInfoView>
where
    R: UserRepo<C>,
    I: ImagePool,
    D: Develop + Send + Sync,
{
    let user_info = GetUserInfo::Id { id: &id }.run_on(repo).await?;

    // Dispatch an activity event when the user reads their own profile.
    if token.user_id == id {
        //
        Event::UserActive(UserActiveEvent {
            user_id: token.user_id,
        })
        .develop_on(develop)
        .await;
    }

    UserInfoView::from_model(image_pool, user_info).await
}

/// Updates a user's QQ ID and nickname.
///
/// Transactional flow:
///
/// 1. **Permission check:** the caller (`token.user_id`) must match the
///    target user (`instr.id`). Returns `Perm` error on mismatch.
/// 2. Updates the user's own record via [`UpdateUser::Info`].
/// 3. Propagates the new nickname to all of the user's memberships via
///    [`UpdateMember::UserNickname`].
///
/// # Type Parameters
///
/// * `N: Nucl<Context = C>` — Transaction coordinator.
/// * `C` — Context anchor.
/// * `R: UserRepo<C> + MemberRepo<C>` — User and member storage.
#[instrument(level = "info", skip(nucl, repo))]
pub async fn update_info<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: UpdateUserInfoInstr,
) -> BaseRest<()>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: UserRepo<C> + MemberRepo<C> + Send + Sync,
{
    // Only the user themselves can update their own profile.
    if token.user_id != instr.id {
        //
        let err_message = trl("error-forbidden");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %token.user_id,
            affected_user_id = %instr.id,
            "expected error: user profile ownership required",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    }

    let user_repl = UserInfoRepl {
        id: token.user_id.clone(),
        qid: instr.qid,
        nickname: instr.nickname,
    };

    let member_repl = MemberNicknameRepl {
        user_id: token.user_id.clone(),
        user_nickname: user_repl.nickname.clone(),
    };

    nucl.coord(async move |context| {
        //
        UpdateUser::Info { repl: &user_repl }
            .step_on(repo, context)
            .await?;

        UpdateMember::UserNickname { repl: &member_repl }
            .step_on(repo, context)
            .await?;

        accept(())
    })
    .await?;

    accept(())
}

/// Replaces a user's password after verifying their current password.
#[instrument(
    level = "info",
    skip(nucl, repo, instr),
    fields(current_password = "[REDACTED]", new_password = "[REDACTED]",)
)]
pub async fn update_password<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    user_id: String,
    instr: UpdateUserPasswordInstr,
) -> BaseRest<()>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: UserRepo<C> + Send + Sync,
{
    if token.user_id != user_id {
        //
        let err_message = trl("error-forbidden");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %token.user_id,
            affected_user_id = %user_id,
            "expected error: password update ownership required",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    }

    let user_info = GetUserInfo::Id { id: &user_id }.run_on(repo).await?;

    let user_credential = GetUserCredential::Qid {
        qid: &user_info.qid,
    }
    .run_on(repo)
    .await?;

    if !UserComplex::verify_password(
        &instr.current_password,
        &user_credential.password_hash,
    )
    .await
    {
        let err_message = trl("error-wrong-credentials");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Auth,
            err_message = %err_message,
            user_id = %user_id,
            qid = %user_info.qid,
            "expected error: current password verification failed",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Auth,
            message: err_message,
        });
    }

    let password_hash = UserComplex::hash_password(&instr.new_password).await?;

    let repl = UserCredsRepl {
        id: user_id.clone(),
        password_hash,
    };

    nucl.coord(async move |context| {
        //
        UpdateUser::PasswordHash { repl: &repl }
            .step_on(repo, context)
            .await?;

        accept(())
    })
    .await?;

    accept(())
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
#[instrument(level = "info", skip(nucl, repo, prom, image_pool))]
pub async fn reserve_avatar<N, C, R, P, I>(
    (nucl, repo, prom, image_pool): (&N, &R, &P, &I),
    token: UserToken,
    instr: ReserveUserAvatarInstr,
) -> BaseRest<ReserveUserAvatarVal>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: UserRepo<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
    I: ImagePool,
{
    ImageComplex::ensure_byte_length(
        instr.new_byte_len,
        image::ResourceKind::UserAvatar,
    )?;

    let (transaction_image_hash, image_ext, new_byte_len) =
        (instr.image_hash.clone(), instr.ext, instr.new_byte_len);

    let (object_key, avatar_version, upload_required) = nucl
        .coord(async move |context| {
            //
            let avatar_reservation = ReserveUserAvatar {
                id: &token.user_id,
                image_hash: &transaction_image_hash,
                image_ext,
            }
            .step_on(repo, context)
            .await?;

            let (mut batch_ids, mut batch_payloads, mut batch_delays) =
                (Vec::new(), Vec::new(), Vec::new());

            if !avatar_reservation.is_upload_required {
                //
                return accept((
                    avatar_reservation.object_key,
                    avatar_reservation.avatar_version,
                    false,
                ));
            }

            if let Some(prev_key) = &avatar_reservation.prev_object_key {
                //
                batch_ids.push(ImageComplex::gen_delete_id());

                batch_payloads.push(TaskPayload::Image(
                    image::ImagePayload::Delete {
                        object_key: prev_key.clone(),
                    },
                ));

                batch_delays.push(None);
            }

            batch_ids.push(ImageComplex::gen_check_id());

            batch_payloads.push(TaskPayload::Image(
                image::ImagePayload::CheckUpload {
                    resource_kind: image::ResourceKind::UserAvatar,
                    resource_id: token.user_id.clone(),
                    object_key: avatar_reservation.object_key.clone(),
                    version: avatar_reservation.avatar_version,
                },
            ));

            batch_delays.push(Some(Duration::from_secs(15 * 60)));

            let batch_tasks = batch_ids
                .iter()
                .zip(batch_payloads.iter())
                .zip(batch_delays.iter())
                .map(|((id, payload), delay)| Task {
                    id,
                    payload,
                    delay: *delay,
                })
                .collect::<Vec<Task<'_, String, TaskPayload>>>();

            DeferBatch::new(&batch_tasks).step_on(prom, context).await?;

            accept((
                avatar_reservation.object_key,
                avatar_reservation.avatar_version,
                true,
            ))
        })
        .await?;

    let slot = match upload_required {
        //
        true => {
            //
            let upload_spec = ImageUploadSpec {
                object_key: &object_key,
                content_type: image_ext.content_type(),
                content_length: new_byte_len,
            };

            let upload_slot = image_pool.get_upload_slot(upload_spec).await?;

            Some(ImageUploadSlotView {
                put_url: upload_slot.url.to_string(),
                image_version: avatar_version,
                headers: upload_slot.headers,
            })
        }

        false => None,
    };

    accept(ReserveUserAvatarVal { slot })
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
#[instrument(level = "info", skip(nucl, repo, image_manager))]
pub async fn mark_avatar_uploaded<N, C, R, I>(
    (nucl, repo, image_manager): (&N, &R, &I),
    token: UserToken,
    id: String,
    instr: MarkUserAvatarUploadedInstr,
) -> BaseRest<()>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: UserRepo<C> + Send + Sync,
    I: ImageManager,
{
    if token.user_id != id {
        //
        let err_message = trl("error-forbidden");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %token.user_id,
            affected_user_id = %id,
            "expected error: avatar upload ownership required",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    }

    let user_info = GetUserInfo::Id { id: &id }.run_on(repo).await?;

    if user_info.avatar_version != Some(instr.image_version) {
        //
        let err_message = trl("error-stale-avatar-upload");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            user_id = %token.user_id,
            affected_user_id = %id,
            image_version = instr.image_version,
            stored_image_version = user_info.avatar_version,
            "expected error: stale user avatar upload",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    if user_info.is_avatar_uploaded == Some(true) {
        return accept(());
    }

    let avatar_key = user_info.avatar_key.clone().ok_or_else(|| {
        //
        let err_message = trl("error-stale-avatar-upload");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            user_id = %token.user_id,
            affected_user_id = %id,
            image_version = instr.image_version,
            stored_image_version = user_info.avatar_version,
            "expected error: stale user avatar upload",
        );

        BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        }
    })?;

    if !image_manager.object_exists(&avatar_key).await? {
        //
        let err_message = trl("error-stale-avatar-upload");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            user_id = %token.user_id,
            affected_user_id = %id,
            image_version = instr.image_version,
            avatar_key = %avatar_key,
            "expected error: stale user avatar upload",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    let repl = UserAvatarRepl {
        id: id.clone(),
        avatar_version: instr.image_version,
        avatar_key: Some(avatar_key.clone()),
        is_avatar_uploaded: true,
    };

    nucl.coord(async move |context| {
        //
        let locked_user_info = GetUserInfoExcluded::Id { id: &id }
            .step_on(repo, context)
            .await?;

        if locked_user_info.avatar_version != Some(instr.image_version)
            || locked_user_info.avatar_key.as_deref() != Some(&avatar_key)
        {
            let err_message = trl("error-stale-avatar-upload");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                user_id = %token.user_id,
                affected_user_id = %id,
                image_version = instr.image_version,
                locked_image_version = locked_user_info.avatar_version,
                avatar_key = %avatar_key,
                "expected error: stale user avatar upload",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        UpdateUser::MarkAvatarUploaded { repl: &repl }
            .step_on(repo, context)
            .await?;

        accept(())
    })
    .await?;

    accept(())
}
