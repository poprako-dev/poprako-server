//! Team use cases — create, read, update, avatar management, and deletion.

use std::time::Duration;

use poprako_orchestra::{
    Nucl, OperRun as _, OperStep as _, run_proxy, step_proxy,
};
use poprako_orchestra_extra::prom::oper::{Defer, DeferBatch};
use poprako_orchestra_extra::prom::task::Task;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::image::ImageComplex;
use crate::complex::member::MemberComplex;
use crate::complex::team::{TeamComplex, TeamPermComplex};
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
use crate::part::prom::Prom;
use crate::part::prom::payload::{TaskPayload, image};
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::assignment_invitation::AssignmentInvitationRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::DeleteAssignments;
use crate::part::repo::oper::assignment_invitation::DeleteAssignmentInvitations;
use crate::part::repo::oper::chapter::{
    DeleteChapter, GetChapterInfoExcluded, ListChapterInfosExcluded,
    UnpinOtherChapters, UpdateChapter,
};
use crate::part::repo::oper::comic::{
    DeleteComic, GetComicInfoExcluded, ListComicInfosExcluded,
    TouchComicLastActive, UpdateComicChapterCount,
};
use crate::part::repo::oper::member::{
    CreateMember, DeleteMember, FindMemberInfo, ListMemberInfosExcluded,
};
use crate::part::repo::oper::page::{DeletePages, ListPageInfos};
use crate::part::repo::oper::team::{
    CreateTeam, DeleteTeam, GetTeamInfo, GetTeamInfoExcluded,
    ReserveTeamAvatar, UpdateTeam,
};
use crate::part::repo::oper::term::DeleteTerms;
use crate::part::repo::oper::termbase::{
    DeleteTermbase, GetTermbaseInfoExcluded, ListTermbaseInfosExcluded,
};
use crate::part::repo::oper::user::{GetUserInfo, GetUserInfoExcluded};
use crate::part::repo::oper::workset::{
    DeleteWorkset, GetWorksetInfoExcluded, ListWorksetInfosExcluded,
    UpdateWorksetComicCount,
};
use crate::part::repo::page::PageRepo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::term::TermRepo;
use crate::part::repo::termbase::TermbaseRepo;
use crate::part::repo::unit::UnitRepo;
use crate::part::repo::user::UserRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::value::role::{RoleField, RoleMask};

pub use self::online::{list_online_user_ids, mark_self_online};
pub use self::read::{get_info, list_infos};

#[cfg(test)]
// Unit and integration tests for team management policies.
mod tests;

// Process-local online-user lease use cases.
mod online;

// Non-transactional team read use cases.
mod read;

/// Creates a new team.
///
/// Transactional — inserts the team and makes the creator an admin member.
///
/// # Type Parameters
///
/// * `C` — Context anchor.
/// * `R: TeamRepo<C>` — Team storage.
/// * `I: ImagePool` — Resolves the avatar signed URL.
#[instrument(level = "info", err(Debug), skip(nucl, repo, image_pool))]
pub async fn create<N, C, R, I>(
    (nucl, repo, image_pool): (&N, &R, &I),
    token: UserToken,
    instr: CreateTeamInstr,
) -> BaseRest<TeamInfoView>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: TeamRepo<C> + UserRepo<C> + MemberRepo<C> + Send + Sync,
    I: ImagePool,
{
    TeamPermComplex::ensure_user_can_create(
        &mut run_proxy! {
            repo => for<'a> GetUserInfo<'a>;
        },
        &token.user_id,
    )
    .await?;

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
#[instrument(level = "info", err(Debug), skip(repo))]
pub async fn update_info<C, R>(
    (repo,): (&R,),
    token: UserToken,
    instr: UpdateTeamInfoInstr,
) -> BaseRest<()>
where
    R: TeamRepo<C> + MemberRepo<C> + Sync,
{
    TeamPermComplex::ensure_user_can_update_info(
        &mut run_proxy! {
            repo => for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &instr.id,
    )
    .await?;

    let repl = TeamRepl {
        id: instr.id.clone(),
        name: instr.name,
        description: instr.description,
    };

    UpdateTeam::Info { repl: &repl }.run_on(repo).await?;

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
    err(Debug),
    skip(nucl, repo, prom, image_pool, token)
)]
pub async fn reserve_avatar<N, C, R, P, I>(
    (nucl, repo, prom, image_pool): (&N, &R, &P, &I),
    token: UserToken,
    id: String,
    instr: ReserveTeamAvatarInstr,
) -> BaseRest<ReserveTeamAvatarVal>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: TeamRepo<C> + MemberRepo<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
    I: ImagePool,
{
    ImageComplex::ensure_byte_length(
        instr.new_byte_len,
        image::ResourceKind::TeamAvatar,
    )?;

    let (transaction_image_hash, image_ext, new_byte_len) =
        (instr.image_hash.clone(), instr.ext, instr.new_byte_len);

    TeamPermComplex::ensure_user_can_reserve_avatar(
        &mut run_proxy! {
            repo => for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &id,
    )
    .await?;

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

                batch_payloads.push(TaskPayload::Image(
                    image::ImagePayload::Delete {
                        object_key: prev_key.clone(),
                    },
                ));

                batch_delays.push(None);
            }

            // Schedule an upload verification check 15 minutes from now.
            batch_ids.push(ImageComplex::gen_check_id());

            batch_payloads.push(TaskPayload::Image(
                image::ImagePayload::CheckUpload {
                    resource_kind: image::ResourceKind::TeamAvatar,
                    resource_id: id.clone(),
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
    // Generate signed URL after commit — the PUT URL should only be issued
    // once the reservation is durable.

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
#[instrument(level = "info", err(Debug), skip(nucl, repo, image_manager))]
pub async fn mark_avatar_uploaded<N, C, R, I>(
    (nucl, repo, image_manager): (&N, &R, &I),
    token: UserToken,
    id: String,
    instr: MarkTeamAvatarUploadedInstr,
) -> BaseRest<()>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: TeamRepo<C> + MemberRepo<C> + Send + Sync,
    I: ImageManager,
{
    TeamPermComplex::ensure_user_can_mark_avatar_uploaded(
        &mut run_proxy! {
            repo => for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &id,
    )
    .await?;

    let team_info = GetTeamInfo::Id { id: &id }.run_on(repo).await?;

    if team_info.avatar_version != instr.image_version {
        //
        let err_message = trl("error-stale-avatar-upload");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            team_id = %id,
            user_id = %token.user_id,
            image_version = instr.image_version,
            stored_image_version = team_info.avatar_version,
            "expected error: stale team avatar upload",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    if team_info.is_avatar_uploaded {
        return accept(());
    }

    let avatar_key = team_info.avatar_key.clone().ok_or_else(|| {
        //
        let err_message = trl("error-stale-avatar-upload");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            team_id = %id,
            user_id = %token.user_id,
            image_version = instr.image_version,
            stored_image_version = team_info.avatar_version,
            "expected error: stale team avatar upload",
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
            team_id = %id,
            user_id = %token.user_id,
            image_version = instr.image_version,
            avatar_key = %avatar_key,
            "expected error: stale team avatar upload",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    let repl = TeamAvatarRepl {
        id: id.clone(),
        avatar_version: instr.image_version,
        avatar_key: Some(avatar_key.clone()),
        is_avatar_uploaded: true,
    };

    nucl.coord(async move |context| {
        //
        let locked_team_info = GetTeamInfoExcluded::Id { id: &id }
            .step_on(repo, context)
            .await?;

        if locked_team_info.avatar_version != instr.image_version
            || locked_team_info.avatar_key.as_deref() != Some(&avatar_key)
        {
            let err_message = trl("error-stale-avatar-upload");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                team_id = %id,
                user_id = %token.user_id,
                image_version = instr.image_version,
                locked_image_version = locked_team_info.avatar_version,
                avatar_key = %avatar_key,
                "expected error: stale team avatar upload",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        UpdateTeam::MarkAvatarUploaded { repl: &repl }
            .step_on(repo, context)
            .await?;

        accept(())
    })
    .await?;

    accept(())
}

/// Deletes a team and all associated instr.
///
/// Transactional cascade:
///
/// 1. Fetches the team info with a pessimistic lock.
/// 2. Lists all worksets belonging to the team.
/// 3. Deletes descendant worksets and comics through their own delete paths.
/// 4. Enqueues avatar deletion if the team had an uploaded avatar.
/// 5. Deletes the team itself.
///
/// # Type Parameters
///
/// * `N: Nucl<Context = C>` — Coordination nucleus.
/// * `C` — Context anchor.
/// * `R: TeamRepo<C> + WorksetRepo<C> + ComicRepo<C>` — Team, workset, and comic storage.
/// * `P: Prom<C>` — Prom enqueuer for deferred avatar deletion.
#[instrument(level = "info", err(Debug), skip(nucl, repo, prom))]
pub async fn delete<N, C, R, P>(
    (nucl, repo, prom): (&N, &R, &P),
    token: UserToken,
    id: String,
) -> BaseRest<()>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: TeamRepo<C>
        + WorksetRepo<C>
        + ComicRepo<C>
        + MemberRepo<C>
        + ChapterRepo<C>
        + PageRepo<C>
        + AssignmentInvitationRepo<C>
        + AssignmentRepo<C>
        + UnitRepo<C>
        + TermbaseRepo<C>
        + TermRepo<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
{
    TeamPermComplex::ensure_user_can_delete(
        &mut run_proxy! {
            repo => for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &id,
    )
    .await?;

    nucl.coord(async move |context| {
        //
        TeamComplex::delete_cascade(
            &mut step_proxy! {
                context;
                repo =>
                    for<'a> GetTeamInfoExcluded<'a>,
                    for<'a> ListWorksetInfosExcluded<'a>,
                    for<'a> DeleteTeam<'a>,
                    for<'a> GetWorksetInfoExcluded<'a>,
                    for<'a> ListComicInfosExcluded<'a>,
                    for<'a> DeleteWorkset<'a>,
                    for<'a, 'b> GetComicInfoExcluded<'a, 'b>,
                    for<'a> ListChapterInfosExcluded<'a>,
                    for<'a> DeleteComic<'a>,
                    for<'a> UpdateWorksetComicCount<'a>,
                    for<'a, 'b> GetChapterInfoExcluded<'a, 'b>,
                    for<'a> ListPageInfos<'a>,
                    for<'a> DeleteAssignmentInvitations<'a>,
                    for<'a> DeleteAssignments<'a>,
                    for<'a> DeletePages<'a>,
                    for<'a> DeleteChapter<'a>,
                    for<'a> UpdateChapter<'a>,
                    for<'a> UnpinOtherChapters<'a>,
                    for<'a> UpdateComicChapterCount<'a>,
                    for<'a> TouchComicLastActive<'a>,
                    for<'a> ListTermbaseInfosExcluded<'a>,
                    for<'a> GetTermbaseInfoExcluded<'a>,
                    for<'a> DeleteTerms<'a>,
                    for<'a> DeleteTermbase<'a>,
                    for<'a> ListMemberInfosExcluded<'a>,
                    for<'a> DeleteMember<'a>;
                prom =>
                    for<'a> Defer<'a, String, TaskPayload, ()>,
                    for<'t, 'a> DeferBatch<'t, 'a, String, TaskPayload, ()>;
            },
            &id,
        )
        .await?;

        accept(())
    })
    .await?;

    accept(())
}
