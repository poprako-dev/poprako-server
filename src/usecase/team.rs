//! Team use cases — create, read, update, avatar management, and deletion.

use std::time::Duration;

use poprako_orchestra::{Nucl, run_proxy, step_proxy};
use poprako_orchestra_extra::prom::oper::{Defer, DeferBatch};
use poprako_orchestra_extra::prom::task::Task;

use tracing::instrument;

use crate::complex::image::ImageComplex;
use crate::complex::member::MemberComplex;
use crate::complex::team::{TeamComplex, TeamPermComplex};
use crate::data::team::{
    CreateTeamParams, ListTeamInfosParams, MarkTeamAvatarUploadedParams,
    ReserveTeamAvatarParams, ReserveTeamAvatarPayload, TeamInfoVal,
    UpdateTeamInfoParams,
};
use crate::model::member::MemberEntry;
use crate::model::team::{TeamEntry, TeamInfo};
use crate::model::user::UserToken;
use crate::part::image::ImagePool;
use crate::part::prom::Prom;
use crate::part::prom::payload::{Payload, image};
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
use crate::part::repo::oper::member::{CreateMember, FindMemberInfo};
use crate::part::repo::oper::page::{DeletePages, ListPageInfos};
use crate::part::repo::oper::team::{
    CreateTeam, DeleteTeam, GetTeamInfo, GetTeamInfoExcluded, ListTeamInfos,
    ReserveTeamAvatar, UpdateTeam,
};
use crate::part::repo::oper::user::{GetUserInfo, GetUserInfoExcluded};
use crate::part::repo::oper::workset::{
    DeleteWorkset, GetWorksetInfoExcluded, ListWorksetInfosExcluded,
    UpdateWorksetComicCount,
};
use crate::part::repo::page::PageRepo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::unit::UnitRepo;
use crate::part::repo::user::UserRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{RegularError, RegularResult};
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
    nucl: &N,
    repo: &R,
    image_pool: &I,
    token: UserToken,
    params: CreateTeamParams,
) -> RegularResult<TeamInfoVal>
where
    N: Nucl<Context = C, Error = RegularError>,
    C: Send,
    R: TeamRepo<C> + UserRepo<C> + MemberRepo<C> + Send + Sync,
    I: ImagePool,
{
    TeamPermComplex::ensure_user_can_list_all(
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
        .coord(async move |context| -> RegularResult<TeamInfo> {
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

            Ok(team_info)
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
    repo: &R,
    image_pool: &I,
    id: String,
) -> RegularResult<TeamInfoVal>
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
    repo: &R,
    image_pool: &I,
    token: UserToken,
    params: ListTeamInfosParams,
) -> RegularResult<Vec<TeamInfoVal>>
where
    R: TeamRepo<C> + UserRepo<C> + Sync,
    I: ImagePool,
{
    if params.user_id.is_none() {
        TeamPermComplex::ensure_user_can_list_all(
            &mut run_proxy! {
                repo => for<'a> GetUserInfo<'a>;
            },
            &token.user_id,
        )
        .await?;
    }

    let team_infos = repo
        .run(&ListTeamInfos {
            user_id: params.user_id.as_deref(),
            offset: params.offset,
            limit: params.limit,
        })
        .await?;

    let team_info_vals = futures_util::future::join_all(
        team_infos
            .into_iter()
            .map(|team_info| TeamInfoVal::from_model(image_pool, team_info)),
    )
    .await
    .into_iter()
    .collect::<RegularResult<Vec<_>>>()?;

    Ok(team_info_vals)
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
    repo: &R,
    token: UserToken,
    params: UpdateTeamInfoParams,
) -> RegularResult<()>
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

    Ok(())
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
    nucl: &N,
    repo: &R,
    prom: &P,
    image_pool: &I,
    token: UserToken,
    id: String,
    params: ReserveTeamAvatarParams,
) -> RegularResult<ReserveTeamAvatarPayload>
where
    N: Nucl<Context = C, Error = RegularError>,
    C: Send,
    R: TeamRepo<C> + MemberRepo<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
    I: ImagePool,
{
    TeamPermComplex::ensure_user_can_reserve_avatar(
        &mut run_proxy! {
            repo => for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &id,
    )
    .await?;

    let (object_key, avatar_version) = nucl
        .coord(async move |context| -> RegularResult<(String, u32)> {
            //
            let avatar_reservation = repo
                .step(
                    context,
                    &ReserveTeamAvatar {
                        id: &id,
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

            // Schedule an upload verification check 15 minutes from now.
            batch_ids.push(ImageComplex::gen_check_id());

            batch_payloads.push(Payload::Image(image::Payload::CheckUpload {
                resource_kind: image::ResourceKind::TeamAvatar,
                resource_id: id.clone(),
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
    // Generate signed URL after commit — the PUT URL should only be issued
    // once the reservation is durable.

    let put_url = image_pool.get_upload_url(&object_key).await?.to_string();

    Ok(ReserveTeamAvatarPayload {
        put_url,
        avatar_version,
    })
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
pub async fn mark_avatar_uploaded<C, R>(
    repo: &R,
    token: UserToken,
    id: String,
    params: MarkTeamAvatarUploadedParams,
) -> RegularResult<()>
where
    R: TeamRepo<C> + MemberRepo<C> + Sync,
{
    TeamPermComplex::ensure_user_can_mark_avatar_uploaded(
        &mut run_proxy! {
            repo => for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &id,
    )
    .await?;

    repo.run(&UpdateTeam::MarkAvatarUploaded {
        id: &id,
        avatar_version: params.avatar_version,
    })
    .await?;

    Ok(())
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
    nucl: &N,
    repo: &R,
    prom: &P,
    token: UserToken,
    id: String,
) -> RegularResult<()>
where
    N: Nucl<Context = C, Error = RegularError>,
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

    nucl.coord(async move |context| -> RegularResult<()> {
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
                    for<'a> TouchComicLastActive<'a>;
                prom =>
                    for<'a> Defer<'a, String, Payload, ()>,
                    for<'t, 'a> DeferBatch<'t, 'a, String, Payload, ()>;
            },
            &id,
        )
        .await?;

        Ok(())
    })
    .await?;

    Ok(())
}
