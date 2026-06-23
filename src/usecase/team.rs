//! Team use cases — create, read, update, avatar management, and deletion.

use time::{Duration, OffsetDateTime};

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;
use poprako_util::page::Page;

use crate::complex::image::ImageComplex;
use crate::complex::team::TeamComplex;
use crate::data::team::{
    CreateTeamData, MarkTeamAvatarUploadedData, ReserveTeamAvatarData, ReserveTeamAvatarVal,
    TeamInfoVal, UpdateTeamInfoData,
};
use crate::model::team::TeamForm;
use crate::part::image::ImagePool;
use crate::part::prom::intention::{IMAGE_TOPIC, ImageIntention, ImageKind};
use crate::part::prom::{Payload, Prom, PromStep, PromTransactional};
use crate::part::repo::map_drive_err;
use crate::part::repo::step::team::TeamStep;
use crate::part::repo::step::workset::WorksetStep;
use crate::part::repo::team::{TeamRepo, TeamRepoTransactional};
use crate::part::repo::workset::{WorksetRepo, WorksetRepoTransactional};
use crate::result::{RootError, RootResult, accept};
use crate::util::DeriveTransactional;

#[cfg(test)]
pub(crate) mod tests;

/// Creates a new team.
///
/// Non-transactional — generates an ID via [`TeamComplex::gen_id`], inserts
/// the row, and returns presentation-ready team info with a resolved avatar URL.
///
/// # Type Parameters
///
/// * `C` — Context anchor.
/// * `R: TeamRepo<C>` — Team storage.
/// * `I: ImagePool` — Resolves the avatar signed URL.
pub async fn create<C, R, I>(repo: &R, image: &I, data: CreateTeamData) -> RootResult<TeamInfoVal>
where
    R: TeamRepo<C>,
    <R as DeriveTransactional>::Transactional: TeamRepoTransactional<C>,
    I: ImagePool,
{
    let form = TeamForm {
        id: TeamComplex::gen_id(),
        name: data.name,
        description: data.description,
    };

    let info = repo.execute(&TeamStep::create(&form)).await?;

    TeamInfoVal::from_model(image, info).await
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
pub async fn get_info<C, R, I>(repo: &R, image: &I, id: String) -> RootResult<TeamInfoVal>
where
    R: TeamRepo<C>,
    <R as DeriveTransactional>::Transactional: TeamRepoTransactional<C>,
    I: ImagePool,
{
    let info = repo.execute(&TeamStep::get_info_by_id(&id)).await?;

    TeamInfoVal::from_model(image, info).await
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
pub async fn list_infos<C, R, I>(repo: &R, image: &I, page: Page) -> RootResult<Vec<TeamInfoVal>>
where
    R: TeamRepo<C>,
    <R as DeriveTransactional>::Transactional: TeamRepoTransactional<C>,
    I: ImagePool,
{
    let team_infos = repo.execute(&TeamStep::list(page)).await?;

    // TODO: join all.
    let mut team_info_vals = Vec::with_capacity(team_infos.len());
    for info in team_infos {
        team_info_vals.push(TeamInfoVal::from_model(image, info).await?);
    }

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
pub async fn update_info<C, R>(repo: &R, data: UpdateTeamInfoData) -> RootResult<()>
where
    R: TeamRepo<C>,
    <R as DeriveTransactional>::Transactional: TeamRepoTransactional<C>,
{
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
/// 2. If replacing an existing avatar, enqueues a [`Delete`](ImageIntention::Delete)
///    prom record (visible immediately) to remove the old object.
/// 3. Enqueues a [`CheckUploaded`](ImageIntention::CheckUploaded) prom record
///    (visible after 15 minutes) to verify the client completed the upload.
///
/// After commit, generates a signed PUT URL for the client to upload to.
///
/// # Type Parameters
///
/// * `D: Drive<C>` — Transaction driver.
/// * `C` — Context anchor.
/// * `R: TeamRepo<C>` — Team storage.
/// * `P: Prom<C>` — Prom enqueuer for deferred image operations.
/// * `I: ImagePool` — Generates the signed upload URL.
pub async fn reserve_avatar<D, C, R, P, I>(
    drive: &D,
    repo: &R,
    prom: &P,
    image: &I,
    id: String,
    data: ReserveTeamAvatarData,
) -> RootResult<ReserveTeamAvatarVal>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: TeamRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: TeamRepoTransactional<C> + Send,
    P: Prom<C> + Send + Sync,
    <P as DeriveTransactional>::Transactional: PromTransactional<C> + Send + Sync,
    I: ImagePool,
{
    let (object_key, avatar_version) = drive
        .with_context(async move |context| {
            let repo = DeriveTransactional::transactional(repo).await;
            let prom = DeriveTransactional::transactional(prom).await;

            let reservation = repo
                .advance(context, &TeamStep::reserve_avatar(&id, &data.file_ext))
                .await?;

            let now = OffsetDateTime::now_utc();

            // If replacing an existing avatar, schedule deletion of the old object.
            if let Some(previous_key) = &reservation.previous_object_key {
                let delete_id = ImageComplex::gen_delete_id();

                prom.advance(
                    context,
                    &PromStep::append(
                        &delete_id,
                        IMAGE_TOPIC,
                        Payload::Image(ImageIntention::Delete {
                            object_key: previous_key.clone(),
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
                    Payload::Image(ImageIntention::CheckUploaded {
                        kind: ImageKind::TeamAvatar,
                        resource_id: id.clone(),
                        object_key: reservation.object_key.clone(),
                        image_version: reservation.avatar_version,
                    }),
                    &check_visible_at,
                ),
            )
            .await?;

            accept((reservation.object_key, reservation.avatar_version))
        })
        .await
        .map_err(map_drive_err)?;

    // Generate signed URL after commit — the PUT URL should only be issued
    // once the reservation is durable.
    let put_url = image.put_signed(&object_key).await?.to_string();

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
    id: String,
    data: MarkTeamAvatarUploadedData,
) -> RootResult<()>
where
    R: TeamRepo<C>,
    <R as DeriveTransactional>::Transactional: TeamRepoTransactional<C>,
{
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
/// 3. Cascade-deletes each workset.
/// 4. Deletes the team itself.
/// 5. If the team had an uploaded avatar, enqueues a prom record to delete
///    the avatar object from storage.
///
/// Requires both [`TeamRepo`] and [`WorksetRepo`] on the repository bound.
///
/// # Type Parameters
///
/// * `D: Drive<C>` — Transaction driver.
/// * `C` — Context anchor.
/// * `R: TeamRepo<C> + WorksetRepo<C>` — Team and workset storage.
/// * `P: Prom<C>` — Prom enqueuer for deferred avatar deletion.
pub async fn delete<D, C, R, P>(drive: &D, repo: &R, prom: &P, id: String) -> RootResult<()>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: TeamRepo<C> + WorksetRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional:
        TeamRepoTransactional<C> + WorksetRepoTransactional<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
    <P as DeriveTransactional>::Transactional: PromTransactional<C> + Send + Sync,
{
    drive
        .with_context(async move |context| {
            let repo = DeriveTransactional::transactional(repo).await;
            let prom = DeriveTransactional::transactional(prom).await;

            let team_info = repo
                .advance(context, &TeamStep::get_info_excluded(&id))
                .await?;

            // Delete all worksets before deleting the team (foreign key ordering).
            let workset_infos = repo
                .advance(context, &WorksetStep::list_by_team_id_excluded(&id))
                .await?;

            for workset in &workset_infos {
                repo.advance(context, &WorksetStep::delete_cascade(&workset.id))
                    .await?;
            }

            repo.advance(context, &TeamStep::delete(&id)).await?;

            // Enqueue avatar object deletion if one was uploaded.
            if let Some(avatar_key) = &team_info.avatar_key
                && team_info.avatar_uploaded
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
        .map_err(map_drive_err)
}
