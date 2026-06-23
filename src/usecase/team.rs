use time::{Duration, OffsetDateTime};

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;
use poprako_util::page::Page;

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
#[cfg(test)]
pub(crate) mod tests;

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

pub async fn get_info<C, R, I>(repo: &R, image: &I, id: String) -> RootResult<TeamInfoVal>
where
    R: TeamRepo<C>,
    <R as DeriveTransactional>::Transactional: TeamRepoTransactional<C>,
    I: ImagePool,
{
    let info = repo.execute(&TeamStep::get_info_by_id(&id)).await?;

    TeamInfoVal::from_model(image, info).await
}

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

            if let Some(previous_key) = &reservation.previous_object_key {
                let delete_id = TeamComplex::gen_avatar_delete_id();

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

            let check_id = TeamComplex::gen_avatar_check_id();
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

    let put_url = image.put_signed(&object_key).await?.to_string();

    Ok(ReserveTeamAvatarVal {
        put_url,
        avatar_version,
    })
}

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

            let workset_infos = repo
                .advance(context, &WorksetStep::list_by_team_id_excluded(&id))
                .await?;

            for workset in &workset_infos {
                repo.advance(context, &WorksetStep::delete_cascade(&workset.id))
                    .await?;
            }

            repo.advance(context, &TeamStep::delete(&id)).await?;

            if let Some(avatar_key) = &team_info.avatar_key
                && team_info.avatar_uploaded
            {
                let now = OffsetDateTime::now_utc();
                let delete_id = TeamComplex::gen_avatar_delete_id();

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
