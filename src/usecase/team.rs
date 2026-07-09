//! Team use cases — create, read, update, avatar management, and deletion.

use time::{Duration, OffsetDateTime};

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;
use poprako_util::page::Page;

use crate::complex::image::ImageComplex;
use crate::complex::member::MemberComplex;
use crate::complex::team::{TeamComplex, TeamPermComplex};
use crate::data::team::{
    CreateTeamData, ListTeamInfosData, MarkTeamAvatarUploadedData,
    ReserveTeamAvatarData, ReserveTeamAvatarVal, TeamInfoVal,
    UpdateTeamInfoData,
};
use crate::model::member::MemberForm;
use crate::model::team::{TeamForm, TeamInfo};
use crate::model::user::UserToken;
use crate::part::image::ImagePool;
use crate::part::prom::task::{IMAGE_TOPIC, ImageKind, ImageTask};
use crate::part::prom::{Payload, Prom, PromStep};
use crate::part::repo::assignment::{
    AssignmentRepo, AssignmentRepoTransactional,
};
use crate::part::repo::assignment_invitation::{
    AssignmentInvitationRepo, AssignmentInvitationRepoTransactional,
};
use crate::part::repo::chapter::{ChapterRepo, ChapterRepoTransactional};
use crate::part::repo::comic::{ComicRepo, ComicRepoTransactional};
use crate::part::repo::member::{MemberRepo, MemberRepoTransactional};
use crate::part::repo::page::{PageRepo, PageRepoTransactional};
use crate::part::repo::step::member::MemberStep;
use crate::part::repo::step::team::TeamStep;
use crate::part::repo::step::user::UserStep;
use crate::part::repo::team::{TeamRepo, TeamRepoTransactional};
use crate::part::repo::unit::{UnitRepo, UnitRepoTransactional};
use crate::part::repo::user::{UserRepo, UserRepoTransactional};
use crate::part::repo::workset::{WorksetRepo, WorksetRepoTransactional};
use crate::result::{RegularError, RegularResult};
use crate::util::DeriveTransactional;
use crate::value::role::{RoleField, RoleMask};

#[cfg(test)]
pub(crate) mod tests;

/// Creates a new team.
///
/// Transactional — inserts the team and makes the creator an admin member.
///
/// # Type Parameters
///
/// * `C` — Context anchor.
/// * `R: TeamRepo<C>` — Team storage.
/// * `I: ImagePool` — Resolves the avatar signed URL.
pub async fn create<D, C, R, I>(
    drive: &D,
    repo: &R,
    image_pool: &I,
    token: UserToken,
    data: CreateTeamData,
) -> RegularResult<TeamInfoVal>
where
    D: Drive<C>,
    D::Error: Into<RegularError>,
    C: Send,
    R: TeamRepo<C> + UserRepo<C> + MemberRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: TeamRepoTransactional<C>
        + UserRepoTransactional<C>
        + MemberRepoTransactional<C>
        + Send
        + Sync,
    I: ImagePool,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    TeamPermComplex::can_user_list_all(&mut repo.as_proxy(), &token.user_id)
        .await?;

    let team_form = TeamForm {
        id: TeamComplex::gen_id(),
        name: data.name,
        description: data.description,
    };

    let team_info: TeamInfo = drive
        .with_context(async move |context| -> RegularResult<TeamInfo> {
            //
            let repo = repo.derive_transactional().await;

            let user_info = repo
                .advance(context, &UserStep::get_info_excluded(&token.user_id))
                .await?;

            let team_info =
                repo.advance(context, &TeamStep::create(&team_form)).await?;

            let member_form = MemberForm {
                id: MemberComplex::gen_id(),
                user_id: token.user_id,
                user_nickname: user_info.nickname,
                team_id: team_info.id.clone(),
                roles: RoleMask::from(RoleField::ADMIN),
            };

            repo.advance(context, &MemberStep::create(&member_form))
                .await?;

            Ok(team_info)
        })
        .await?;
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
pub async fn get_info<C, R, I>(
    repo: &R,
    image_pool: &I,
    id: String,
) -> RegularResult<TeamInfoVal>
where
    R: TeamRepo<C>,
    <R as DeriveTransactional>::Transactional: TeamRepoTransactional<C>,
    I: ImagePool,
{
    TeamInfoVal::from_model(
        image_pool,
        repo.execute(&TeamStep::get_info_by_id(&id)).await?,
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
pub async fn list_infos<C, R, I>(
    repo: &R,
    image_pool: &I,
    token: UserToken,
    data: ListTeamInfosData,
) -> RegularResult<Vec<TeamInfoVal>>
where
    R: TeamRepo<C> + UserRepo<C> + Sync,
    <R as DeriveTransactional>::Transactional:
        TeamRepoTransactional<C> + UserRepoTransactional<C>,
    I: ImagePool,
{
    if data.user_id.is_none() {
        // TODO: comment
        use crate::part::shared::proxy::AsProxyNonTransactional as _;

        TeamPermComplex::can_user_list_all(
            &mut repo.as_proxy(),
            &token.user_id,
        )
        .await?;
    }

    let team_infos = repo
        .execute(&TeamStep::list_infos(
            data.user_id.as_deref(),
            Page {
                offset: data.offset,
                limit: data.limit,
            },
        ))
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
pub async fn update_info<C, R>(
    repo: &R,
    token: UserToken,
    data: UpdateTeamInfoData,
) -> RegularResult<()>
where
    R: TeamRepo<C> + MemberRepo<C> + Sync,
    <R as DeriveTransactional>::Transactional:
        TeamRepoTransactional<C> + MemberRepoTransactional<C>,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    TeamPermComplex::can_user_update_info(
        &mut repo.as_proxy(),
        &token.user_id,
        &data.id,
    )
    .await?;

    repo.execute(&TeamStep::update_info(
        &data.id,
        &data.name,
        &data.description,
    ))
    .await?;

    Ok(())
}

/// Reserves a new avatar upload slot for a team.
///
/// Transactional flow:
///
/// 1. Calls [`TeamStep::reserve_avatar`] — updates the avatar key, increments
///    the version, and returns any previous avatar key for cleanup.
/// 2. If replacing an existing avatar, enqueues a [`Delete`](ImageTask::Delete)
///    prom record (visible immediately) to remove the old object.
/// 3. Enqueues a [`CheckUploaded`](ImageTask::CheckUploaded) prom record
///    (visible after 15 minutes) to verify the client completed the upload.
///
/// After commit, generates a signed PUT URL for the client to upload to.
///
/// # Type Parameters
///
/// * `D: Drive<C>` — Transaction driver.
/// * `C` — Context anchor.
/// * `R: TeamRepo<C>` — Team storage.
/// * `P: PromTransactional<C>` — Prom enqueuer for deferred image opers.
/// * `I: ImagePool` — Generates the signed upload URL.
pub async fn reserve_avatar<D, C, R, P, I>(
    drive: &D,
    repo: &R,
    prom: &P,
    image_pool: &I,
    token: UserToken,
    id: String,
    data: ReserveTeamAvatarData,
) -> RegularResult<ReserveTeamAvatarVal>
where
    D: Drive<C>,
    D::Error: Into<RegularError>,
    C: Send,
    R: TeamRepo<C> + MemberRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional:
        TeamRepoTransactional<C> + MemberRepoTransactional<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
    I: ImagePool,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    TeamPermComplex::can_user_reserve_avatar(
        &mut repo.as_proxy(),
        &token.user_id,
        &id,
    )
    .await?;

    let (object_key, avatar_version) = drive
        .with_context(async move |context| -> RegularResult<(String, i64)> {
            //
            let repo = repo.derive_transactional().await;

            let avatar_reservation = repo
                .advance(
                    context,
                    &TeamStep::reserve_avatar(&id, &data.file_ext),
                )
                .await?;

            let now = OffsetDateTime::now_utc();

            // If replacing an existing avatar, schedule deletion of the old object.
            if let Some(prev_key) = &avatar_reservation.prev_object_key {
                //
                let delete_id = ImageComplex::gen_delete_id();

                prom.advance(
                    context,
                    &PromStep::append(
                        &delete_id,
                        IMAGE_TOPIC,
                        Payload::Image(ImageTask::Delete {
                            object_key: prev_key.as_str(),
                        }),
                        &now,
                    ),
                )
                .await?;
            }

            // Schedule an upload verification check 15 minutes from now.
            let check_id = ImageComplex::gen_check_id();

            let check_visible_at = now + Duration::minutes(15);

            prom.advance(
                context,
                &PromStep::append(
                    &check_id,
                    IMAGE_TOPIC,
                    Payload::Image(ImageTask::CheckUploaded {
                        kind: ImageKind::TeamAvatar,
                        resource_id: &id,
                        object_key: &avatar_reservation.object_key,
                        image_version: avatar_reservation.avatar_version,
                    }),
                    &check_visible_at,
                ),
            )
            .await?;

            Ok((
                avatar_reservation.object_key,
                avatar_reservation.avatar_version,
            ))
        })
        .await?;
    // Generate signed URL after commit — the PUT URL should only be issued
    // once the reservation is durable.
    let put_url = image_pool.put_signed(&object_key).await?.to_string();

    Ok(ReserveTeamAvatarVal {
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
pub async fn mark_avatar_uploaded<C, R>(
    repo: &R,
    token: UserToken,
    id: String,
    data: MarkTeamAvatarUploadedData,
) -> RegularResult<()>
where
    R: TeamRepo<C> + MemberRepo<C> + Sync,
    <R as DeriveTransactional>::Transactional:
        TeamRepoTransactional<C> + MemberRepoTransactional<C>,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    TeamPermComplex::can_user_mark_avatar_uploaded(
        &mut repo.as_proxy(),
        &token.user_id,
        &id,
    )
    .await?;

    repo.execute(&TeamStep::mark_avatar_uploaded(&id, data.avatar_version))
        .await?;

    Ok(())
}

/// Deletes a team and all associated data.
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
/// * `D: Drive<C>` — Transaction driver.
/// * `C` — Context anchor.
/// * `R: TeamRepo<C> + WorksetRepo<C> + ComicRepo<C>` — Team, workset, and comic storage.
/// * `P: PromTransactional<C>` — Prom enqueuer for deferred avatar deletion.
pub async fn delete<D, C, R, P>(
    drive: &D,
    repo: &R,
    prom: &P,
    token: UserToken,
    id: String,
) -> RegularResult<()>
where
    D: Drive<C>,
    D::Error: Into<RegularError>,
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
    <R as DeriveTransactional>::Transactional:
        TeamRepoTransactional<C>
            + WorksetRepoTransactional<C>
            + ComicRepoTransactional<C>
            + MemberRepoTransactional<C>
            + ChapterRepoTransactional<C>
            + PageRepoTransactional<C>
            + AssignmentInvitationRepoTransactional<C>
            + AssignmentRepoTransactional<C>
            + UnitRepoTransactional<C>
            + Send
            + Sync,
    P: Prom<C> + Send + Sync,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    TeamPermComplex::can_user_delete(&mut repo.as_proxy(), &token.user_id, &id)
        .await?;

    drive
        .with_context(async move |context| -> RegularResult<()> {
            //
            let repo = repo.derive_transactional().await;

            TeamComplex::delete_cascade(&repo, prom, context, &id).await?;

            Ok(())
        })
        .await?;
    Ok(())
}
