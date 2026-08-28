//! Team use cases — create, read, update, avatar management, and deletion.

/// Team deletion orchestration.
pub mod delete;
/// Process-local online-user lease use cases.
pub mod online;
/// Non-transactional team read use cases.
pub mod read;

#[cfg(test)]
// Unit and integration tests for team management policies.
mod tests;

use std::time::Duration;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::image::ImageComplex;
use crate::complex::member::MemberComplex;
use crate::complex::team::{TeamComplex, TeamPermComplex};
use crate::config::ImageConfig;
use crate::data::instr::team::{
    CreateTeamInstr, MarkTeamAvatarUploadedInstr, ReserveTeamAvatarInstr,
    UpdateTeamInfoInstr,
};
use crate::data::val::team::ReserveTeamAvatarVal;
use crate::data::view::image::ImageUploadSlotView;
use crate::data::view::team::TeamInfoView;
use crate::model::shared::user::UserToken;
use crate::model::write::member::MemberEntry;
use crate::model::write::team::{TeamAvatarRepl, TeamEntry, TeamRepl};
use crate::part::image::{ImageManager, ImagePool, ImageUploadSpec};
use crate::part::nucl::ReptRead;
use crate::part::prom::Prom;
use crate::part::prom::oper::DeferBatch;
use crate::part::prom::payload::{TaskPayload, image};
use crate::part::prom::task::Task;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::member::{CreateMember, FindMemberInfo};
use crate::part::repo::oper::team::{
    CreateTeam, GetTeamInfo, GetTeamInfoExcluded, ReserveTeamAvatar, UpdateTeam,
};
use crate::part::repo::oper::user::{GetUserInfo, GetUserInfoExcluded};
use crate::part::repo::team::TeamRepo;
use crate::part::repo::user::UserRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::value::image::ImageKind;
use crate::value::role::{RoleField, RoleMask};

/// Creates a new team.
///
/// Transactional — inserts the team and makes the creator an admin member.
///
/// # Type Parameters
///
/// * `C` — Context anchor.
/// * `R: TeamRepo<C>` — Team storage.
/// * `I: ImagePool` — Resolves the avatar signed URL.
#[instrument(level = "info", skip(nucl, repo, image_pool))]
pub async fn create<N, C, R, I>(
    (nucl, repo, image_pool): (&N, &R, &I),
    token: UserToken,
    instr: CreateTeamInstr,
) -> BaseRest<TeamInfoView>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: TeamRepo<C> + UserRepo<C> + MemberRepo<C> + Send + Sync,
    I: ImagePool + Sync,
{
    let user_info = GetUserInfo::Id { id: &token.user_id }.run_on(repo).await?;

    TeamPermComplex::ensure_user_can_create(&user_info)?;

    let team_entry = TeamEntry {
        id: TeamComplex::gen_id(),
        name: instr.name,
        description: instr.description,
    };

    let team_info = nucl
        .coord(async move |context| {
            //
            let user_info = GetUserInfoExcluded::Id { id: &token.user_id }
                .step_on(repo, context)
                .await?;

            let team_info = CreateTeam { entry: &team_entry }
                .step_on(repo, context)
                .await?;

            let member_entry = MemberEntry {
                id: MemberComplex::gen_id(),
                user_id: token.user_id,
                user_nickname: user_info.nickname,
                team_id: team_info.id.clone(),
                roles: RoleMask::from(RoleField::ADMIN),
            };

            CreateMember {
                entry: &member_entry,
            }
            .step_on(repo, context)
            .await?;

            accept(team_info)
        })
        .await?;

    // FIXME: no need to use info val in create.
    TeamInfoView::from_model(image_pool, team_info).await
}

/// Updates a team's name and description.
///
/// Non-transactional single-row update.
///
/// # Type Parameters
///
/// * `C` — Context anchor.
/// * `R: TeamRepo<C>` — Team storage.
#[instrument(level = "info", skip(repo))]
pub async fn update_info<C, R>(
    (repo,): (&R,),
    token: UserToken,
    instr: UpdateTeamInfoInstr,
) -> BaseRest<()>
where
    C: Context,
    R: TeamRepo<C> + MemberRepo<C> + Sync,
{
    let member_info = FindMemberInfo::UserTeam {
        user_id: &token.user_id,
        team_id: &instr.id,
    }
    .run_on(repo)
    .await?;

    let Some(member_info) = member_info else {
        //
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-team-admin-required"),
        });
    };

    TeamPermComplex::ensure_user_can_update_info(&member_info)?;

    let team_repl = TeamRepl {
        id: instr.id,
        name: instr.name,
        description: instr.description,
    };

    UpdateTeam::Info { repl: &team_repl }.run_on(repo).await?;

    accept(())
}

/// Reserves a new avatar upload slot for a team.
///
/// Transactional flow:
///
/// 1. Calls [`ReserveTeamAvatar`] — updates the avatar key, increments
///    the version, and returns any previous avatar key for cleanup.
/// 2. If replacing an existing avatar, defers an immediate image-delete payload.
/// 3. Defers an image upload-check payload with a 15-minute delay.
///
/// After commit, generates a signed PUT URL for the client to upload to.
///
/// # Type Parameters
///
/// * `N: Nucl<Context = C>` — Coordination nucleus.
/// * `C` — Context anchor.
/// * `R: TeamRepo<C>` — Team storage.
/// * `P: Prom<C>` — Prom enqueuer for deferred image opers.
/// * `I: ImagePool` — Generates the signed upload URL.
#[instrument(
    level = "info",
    skip(nucl, repo, prom, image_pool, image_config, token)
)]
pub async fn reserve_avatar<N, C, R, P, I>(
    (nucl, repo, prom, image_pool, image_config): (
        &N,
        &R,
        &P,
        &I,
        &ImageConfig,
    ),
    token: UserToken,
    id: String,
    instr: ReserveTeamAvatarInstr,
) -> BaseRest<ReserveTeamAvatarVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: TeamRepo<C> + MemberRepo<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
    I: ImagePool + Sync,
{
    ImageComplex::ensure_byte_length(
        image_config,
        instr.new_byte_len,
        ImageKind::TeamAvatar,
    )?;

    let (transaction_image_hash, image_ext, new_byte_len) =
        (instr.image_hash, instr.ext, instr.new_byte_len);

    let member_info = FindMemberInfo::UserTeam {
        user_id: &token.user_id,
        team_id: &id,
    }
    .run_on(repo)
    .await?;

    let Some(member_info) = member_info else {
        //
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-team-admin-required"),
        });
    };

    TeamPermComplex::ensure_user_can_reserve_avatar(&member_info)?;

    let (object_key, avatar_version, upload_required) = nucl
        .coord(async move |context| {
            //
            let avatar_reservation = ReserveTeamAvatar {
                id: &id,
                image_hash: &transaction_image_hash,
                image_ext,
            }
            .step_on(repo, context)
            .await?;

            if !avatar_reservation.is_upload_required {
                //
                return accept((
                    avatar_reservation.object_key,
                    avatar_reservation.avatar_version,
                    false,
                ));
            }

            let (mut batch_ids, mut batch_payloads, mut batch_delays) =
                (Vec::new(), Vec::new(), Vec::new());

            // If replacing an existing avatar, schedule deletion of the old object.
            if let Some(prev_key) = &avatar_reservation.prev_object_key {
                //
                batch_ids.push(ImageComplex::gen_delete_id());

                batch_payloads.push(TaskPayload::Image {
                    payload: image::ImagePayload::Delete {
                        object_key: prev_key.clone(),
                    },
                });

                batch_delays.push(None);
            }

            // Schedule an upload verification check 15 minutes from now.
            batch_ids.push(ImageComplex::gen_check_id());

            batch_payloads.push(TaskPayload::Image {
                payload: image::ImagePayload::CheckUpload {
                    image_kind: ImageKind::TeamAvatar,
                    resource_id: id.clone(),
                    object_key: avatar_reservation.object_key.clone(),
                    version: avatar_reservation.avatar_version,
                },
            });

            batch_delays.push(Some(Duration::from_mins(15)));

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
    // Generate signed URL after commit — the PUT URL should only be issued
    // once the reservation is durable.

    let slot = if upload_required {
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
    } else {
        None
    };

    accept(ReserveTeamAvatarVal { slot })
}

/// Marks a reserved team avatar as successfully uploaded.
///
/// Non-transactional — the `avatar_version` must match the version
/// returned by [`reserve_avatar`], otherwise the step rejects the request.
///
/// # Type Parameters
///
/// * `C` — Context anchor.
/// * `R: TeamRepo<C>` — Team storage.
#[instrument(level = "info", skip(nucl, repo, image_manager))]
pub async fn mark_avatar_uploaded<N, C, R, I>(
    (nucl, repo, image_manager): (&N, &R, &I),
    token: UserToken,
    id: String,
    instr: MarkTeamAvatarUploadedInstr,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: TeamRepo<C> + MemberRepo<C> + Send + Sync,
    I: ImageManager,
{
    let avatar_key = prepare_avatar_upload::<C, R, I>(
        repo,
        image_manager,
        &token,
        &id,
        instr.image_version,
    )
    .await?;

    let Some(avatar_key) = avatar_key else {
        return accept(());
    };

    let team_avatar_repl = TeamAvatarRepl {
        id,
        avatar_version: instr.image_version,
        avatar_key: Some(avatar_key),
        is_avatar_uploaded: true,
    };

    nucl.coord(async move |context| {
        //
        let locked_team_info = GetTeamInfoExcluded::Id {
            id: &team_avatar_repl.id,
        }
        .step_on(repo, context)
        .await?;

        if locked_team_info.avatar_version != Some(instr.image_version)
            || locked_team_info.avatar_key.as_deref()
                != team_avatar_repl.avatar_key.as_deref()
        {
            let err_message = trl("error-stale-avatar-upload");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                team_id = %team_avatar_repl.id,
                user_id = %token.user_id,
                image_version = instr.image_version,
                locked_image_version = locked_team_info.avatar_version,
                avatar_key = ?team_avatar_repl.avatar_key,
                "expected error: stale team avatar upload",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        UpdateTeam::MarkAvatarUploaded {
            repl: &team_avatar_repl,
        }
        .step_on(repo, context)
        .await?;

        accept(())
    })
    .await?;

    accept(())
}

// Loads and validates the team avatar upload state.
async fn prepare_avatar_upload<C, R, I>(
    repo: &R,
    image_manager: &I,
    token: &UserToken,
    id: &str,
    image_version: u32,
) -> BaseRest<Option<String>>
where
    C: Context,
    R: TeamRepo<C> + MemberRepo<C>,
    I: ImageManager,
{
    let member_info = FindMemberInfo::UserTeam {
        user_id: &token.user_id,
        team_id: id,
    }
    .run_on(repo)
    .await?;

    let Some(member_info) = member_info else {
        //
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-team-admin-required"),
        });
    };

    TeamPermComplex::ensure_user_can_mark_avatar_uploaded(&member_info)?;

    let team_info = GetTeamInfo::Id { id }.run_on(repo).await?;

    if team_info.avatar_version != Some(image_version) {
        //
        let err_message = trl("error-stale-avatar-upload");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            team_id = %id,
            user_id = %token.user_id,
            image_version,
            stored_image_version = team_info.avatar_version,
            "expected error: stale team avatar upload",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    if team_info.is_avatar_uploaded == Some(true) {
        return accept(None);
    }

    let avatar_key = team_info.avatar_key.clone().ok_or_else(|| {
        //
        let err_message = trl("error-stale-avatar-upload");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            team_id = %id,
            user_id = %token.user_id,
            image_version,
            stored_image_version = team_info.avatar_version,
            "expected error: stale team avatar upload",
        );

        BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        }
    })?;

    if image_manager.object_exists(&avatar_key).await? {
        return accept(Some(avatar_key));
    }

    let err_message = trl("error-stale-avatar-upload");

    tracing::warn!(
        err_variant = ?ExpectedVariant::Args,
        err_message = %err_message,
        team_id = %id,
        user_id = %token.user_id,
        image_version,
        avatar_key = %avatar_key,
        "expected error: stale team avatar upload",
    );

    Err(BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: err_message,
    })
}
