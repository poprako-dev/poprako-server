//! Team use cases — create, read, update, avatar management, and deletion.

use std::time::Duration;

use poprako_orchestra::{Nucl, run_proxy, step_proxy};
use poprako_orchestra_extra::prom::oper::{Defer, DeferBatch};
use poprako_orchestra_extra::prom::task::Task;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::image::ImageComplex;
use crate::complex::member::MemberComplex;
use crate::complex::team::{TeamComplex, TeamPermComplex};
use crate::data::image::ImageUploadSlotVal;
use crate::data::team::{CreateTeamParams, ListTeamInfosParams, MarkTeamAvatarUploadedParams, ReserveTeamAvatarParams, ReserveTeamAvatarPayload, TeamInfoVal, UpdateTeamInfoParams};
use crate::model::member::MemberEntry;
use crate::model::team::{TeamEntry, TeamInfo, TeamInfoListKind, TeamInfoListSpec};
use crate::model::user::UserToken;
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
use crate::part::repo::oper::chapter::{DeleteChapter, GetChapterInfoExcluded, ListChapterInfosExcluded, UnpinOtherChapters, UpdateChapter};
use crate::part::repo::oper::comic::{DeleteComic, GetComicInfoExcluded, ListComicInfosExcluded, TouchComicLastActive, UpdateComicChapterCount};
use crate::part::repo::oper::member::{CreateMember, DeleteMember, FindMemberInfo, ListMemberInfosExcluded};
use crate::part::repo::oper::page::{DeletePages, ListPageInfos};
use crate::part::repo::oper::team::{CreateTeam, DeleteTeam, GetTeamInfo, GetTeamInfoExcluded, ListTeamInfos, ReserveTeamAvatar, UpdateTeam};
use crate::part::repo::oper::term::DeleteTerms;
use crate::part::repo::oper::termbase::{DeleteTermbase, GetTermbaseInfoExcluded, ListTermbaseInfosExcluded};
use crate::part::repo::oper::user::{GetUserInfo, GetUserInfoExcluded};
use crate::part::repo::oper::workset::{DeleteWorkset, GetWorksetInfoExcluded, ListWorksetInfosExcluded, UpdateWorksetComicCount};
use crate::part::repo::page::PageRepo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::term::TermRepo;
use crate::part::repo::termbase::TermbaseRepo;
use crate::part::repo::unit::UnitRepo;
use crate::part::repo::user::UserRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};
use crate::value::role::{RoleField, RoleMask};

#[cfg(test)]
mod tests;

/// Creates a new team.
///
/// Transactional — inserts the team and makes the creator an admin member.
///
/// # Type Parameters
///
/// * `C` — Context anchor.
/// * `R: TeamRepo<C>` — Team storage.
/// * `I: ImagePool` — Resolves the avatar signed URL.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn create<N, C, R, I>(
    (nucl, repo, image_pool): (&N, &R, &I),
    token: UserToken,
    params: CreateTeamParams,
) -> BaseResult<TeamInfoVal>
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
        name: params.name,
        description: params.description,
    };

    let team_info: TeamInfo = nucl
        .coord(async move |context| {
            //
            let user_info = repo
                .step(context, &GetUserInfoExcluded::Id { id: &token.user_id })
                .await?;

            let team_info = repo
                .step(context, &CreateTeam { entry: &team_entry })
                .await?;

            let member_entry = MemberEntry {
                id: MemberComplex::gen_id(),
                user_id: token.user_id,
                user_nickname: user_info.nickname,
                team_id: team_info.id.clone(),
                roles: RoleMask::from(RoleField::ADMIN),
            };

            repo.step(
                context,
                &CreateMember {
                    entry: &member_entry,
                },
            )
            .await?;

            accept(team_info)
        })
        .await?;

    // FIXME: no need to use info val in create.
    TeamInfoVal::from_model(image_pool, team_info).await
}

/// Fetches a team by ID with avatar URL resolution.
///
/// Non-transactional read.
///
/// # Type Parameters
///
/// * `C` — Context anchor.
/// * `R: TeamRepo<C>` — Team storage.
/// * `I: ImagePool` — Resolves the avatar signed URL.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn get_info<C, R, I>(
    (repo, image_pool): (&R, &I),
    id: String,
) -> BaseResult<TeamInfoVal>
where
    R: TeamRepo<C>,
    I: ImagePool,
{
    TeamInfoVal::from_model(
        image_pool,
        repo.run(&GetTeamInfo::Id { id: &id }).await?,
    )
    .await
}

/// Lists teams with pagination.
///
/// Non-transactional read. Each team's avatar URL is resolved individually.
///
/// # Type Parameters
///
/// * `C` — Context anchor.
/// * `R: TeamRepo<C>` — Team storage.
/// * `I: ImagePool` — Resolves avatar signed URLs.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn list_infos<C, R, I>(
    (repo, image_pool): (&R, &I),
    token: UserToken,
    // FIXME: use try_into()?
    params: ListTeamInfosParams,
) -> BaseResult<Vec<TeamInfoVal>>
where
    R: TeamRepo<C> + UserRepo<C> + Sync,
    I: ImagePool,
{
    let kind = match params.user_id {
        //
        Some(user_id) => {
            //
            if user_id != token.user_id {
                todo!()
            }

            TeamInfoListKind::JoinedBy { user_id }
        }

        None => {
            //
            TeamPermComplex::ensure_user_can_list_infos(
                &mut run_proxy! {
                    repo => for<'a> GetUserInfo<'a>;
                },
                &token.user_id,
            )
            .await?;

            TeamInfoListKind::All
        }
    };

    let team_info_list_spec = TeamInfoListSpec {
        kind,
        offset: params.offset,
        limit: params.limit,
    };

    let team_infos = repo
        .run(&ListTeamInfos {
            spec: &team_info_list_spec,
        })
        .await?;

    let team_info_vals = futures_util::future::join_all(
        team_infos
            .into_iter()
            .map(|team_info| TeamInfoVal::from_model(image_pool, team_info)),
    )
    .await
    .into_iter()
    .collect::<BaseResult<Vec<_>>>()?;

    accept(team_info_vals)
}

/// Updates a team's name and description.
///
/// Non-transactional single-row update.
///
/// # Type Parameters
///
/// * `C` — Context anchor.
/// * `R: TeamRepo<C>` — Team storage.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn update_info<C, R>(
    (repo,): (&R,),
    token: UserToken,
    params: UpdateTeamInfoParams,
) -> BaseResult<()>
where
    R: TeamRepo<C> + MemberRepo<C> + Sync,
{
    TeamPermComplex::ensure_user_can_update_info(
        &mut run_proxy! {
            repo => for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &params.id,
    )
    .await?;

    // FIXME: use TeamInfoUpdate instead.

    repo.run(&UpdateTeam::Info {
        id: &params.id,
        name: &params.name,
        description: &params.description,
    })
    .await?;

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
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn reserve_avatar<N, C, R, P, I>(
    (nucl, repo, prom, image_pool): (&N, &R, &P, &I),
    token: UserToken,
    id: String,
    params: ReserveTeamAvatarParams,
) -> BaseResult<ReserveTeamAvatarPayload>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: TeamRepo<C> + MemberRepo<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
    I: ImagePool,
{
    ImageComplex::ensure_byte_length(
        params.new_byte_len,
        image::ResourceKind::TeamAvatar,
    )?;

    let transaction_image_hash = params.image_hash.clone();

    let image_ext = params.ext;

    let new_byte_len = params.new_byte_len;

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
            let avatar_reservation = repo
                .step(
                    context,
                    &ReserveTeamAvatar {
                        id: &id,
                        image_hash: &transaction_image_hash,
                        image_ext,
                    },
                )
                .await?;

            if !avatar_reservation.upload_required {
                return accept((
                    avatar_reservation.object_key,
                    avatar_reservation.avatar_version,
                    false,
                ));
            }

            let mut batch_ids = Vec::new();

            let mut batch_payloads = Vec::new();

            let mut batch_delays = Vec::new();

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

            let batch_tasks: Vec<Task<'_, String, TaskPayload>> = batch_ids
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

            Some(ImageUploadSlotVal {
                put_url: upload_slot.url.to_string(),
                image_version: avatar_version,
                headers: upload_slot.headers,
            })
        }

        false => None,
    };

    accept(ReserveTeamAvatarPayload { slot })
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
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn mark_avatar_uploaded<N, C, R, I>(
    (nucl, repo, image_manager): (&N, &R, &I),
    token: UserToken,
    id: String,
    params: MarkTeamAvatarUploadedParams,
) -> BaseResult<()>
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

    let team_info = repo.run(&GetTeamInfo::Id { id: &id }).await?;

    if team_info.avatar_version != params.image_version {
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: trl("error-stale-avatar-upload"),
        });
    }

    if team_info.avatar_uploaded {
        return accept(());
    }

    let avatar_key =
        team_info
            .avatar_key
            .clone()
            .ok_or_else(|| BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-stale-avatar-upload"),
            })?;

    if !image_manager.object_exists(&avatar_key).await? {
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: trl("error-stale-avatar-upload"),
        });
    }

    nucl.coord(async move |context| {
        //
        let locked_team_info = repo
            .step(context, &GetTeamInfoExcluded::Id { id: &id })
            .await?;

        if locked_team_info.avatar_version != params.image_version
            || locked_team_info.avatar_key.as_deref() != Some(&avatar_key)
        {
            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-stale-avatar-upload"),
            });
        }

        repo.step(
            context,
            &UpdateTeam::MarkAvatarUploaded {
                id: &id,
                avatar_version: params.image_version,
                avatar_key: Some(&avatar_key),
                avatar_uploaded: true,
            },
        )
        .await?;

        accept(())
    })
    .await?;

    accept(())
}

/// Deletes a team and all associated params.
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
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn delete<N, C, R, P>(
    (nucl, repo, prom): (&N, &R, &P),
    token: UserToken,
    id: String,
) -> BaseResult<()>
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
