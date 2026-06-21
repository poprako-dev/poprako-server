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
use crate::part::query::map_drive_err;
use crate::part::query::step::team::TeamStep;
use crate::part::query::step::workset::WorksetStep;
use crate::part::query::team::{TeamQuery, TeamQueryTransactional};
use crate::part::query::workset::{WorksetQuery, WorksetQueryTransactional};
use crate::result::{RootError, RootResult, accept};
use crate::util::DeriveTransactional;

pub async fn create<C, Q, I>(query: &Q, image: &I, data: CreateTeamData) -> RootResult<TeamInfoVal>
where
    Q: TeamQuery<C>,
    <Q as DeriveTransactional>::Transactional: TeamQueryTransactional<C>,
    I: ImagePool,
{
    let form = TeamForm {
        id: TeamComplex::gen_id(),
        name: data.name,
        description: data.description,
    };

    let info = query.execute(&TeamStep::create(&form)).await?;

    TeamInfoVal::from_model(image, info).await
}

pub async fn get_info<C, Q, I>(query: &Q, image: &I, id: String) -> RootResult<TeamInfoVal>
where
    Q: TeamQuery<C>,
    <Q as DeriveTransactional>::Transactional: TeamQueryTransactional<C>,
    I: ImagePool,
{
    let info = query.execute(&TeamStep::get_info_by_id(&id)).await?;

    TeamInfoVal::from_model(image, info).await
}

pub async fn list_infos<C, Q, I>(query: &Q, image: &I, page: Page) -> RootResult<Vec<TeamInfoVal>>
where
    Q: TeamQuery<C>,
    <Q as DeriveTransactional>::Transactional: TeamQueryTransactional<C>,
    I: ImagePool,
{
    let team_infos = query.execute(&TeamStep::list(page)).await?;

    // TODO: join all.
    let mut team_info_vals = Vec::with_capacity(team_infos.len());
    for info in team_infos {
        team_info_vals.push(TeamInfoVal::from_model(image, info).await?);
    }

    Ok(team_info_vals)
}

pub async fn update_info<C, Q>(query: &Q, data: UpdateTeamInfoData) -> RootResult<()>
where
    Q: TeamQuery<C>,
    <Q as DeriveTransactional>::Transactional: TeamQueryTransactional<C>,
{
    query
        .execute(&TeamStep::update_info(
            &data.id,
            &data.name,
            &data.description,
        ))
        .await?;

    Ok(())
}

pub async fn reserve_avatar<D, C, Q, P, I>(
    drive: &D,
    query: &Q,
    prom: &P,
    image: &I,
    id: String,
    data: ReserveTeamAvatarData,
) -> RootResult<ReserveTeamAvatarVal>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    Q: TeamQuery<C> + Send + Sync,
    <Q as DeriveTransactional>::Transactional: TeamQueryTransactional<C> + Send,
    P: Prom<C> + Send + Sync,
    <P as DeriveTransactional>::Transactional: PromTransactional<C> + Send + Sync,
    I: ImagePool,
{
    let (object_key, avatar_version) = drive
        .with_context(async move |context| {
            let query = DeriveTransactional::transactional(query).await;
            let prom = DeriveTransactional::transactional(prom).await;

            let reservation = query
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

pub async fn mark_avatar_uploaded<C, Q>(
    query: &Q,
    id: String,
    data: MarkTeamAvatarUploadedData,
) -> RootResult<()>
where
    Q: TeamQuery<C>,
    <Q as DeriveTransactional>::Transactional: TeamQueryTransactional<C>,
{
    query
        .execute(&TeamStep::mark_avatar_uploaded(&id, data.avatar_version))
        .await?;

    Ok(())
}

pub async fn delete<D, C, Q, P>(drive: &D, query: &Q, prom: &P, id: String) -> RootResult<()>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    Q: TeamQuery<C> + WorksetQuery<C> + Send + Sync,
    <Q as DeriveTransactional>::Transactional:
        TeamQueryTransactional<C> + WorksetQueryTransactional<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
    <P as DeriveTransactional>::Transactional: PromTransactional<C> + Send + Sync,
{
    drive
        .with_context(async move |context| {
            let query = DeriveTransactional::transactional(query).await;
            let prom = DeriveTransactional::transactional(prom).await;

            let team_info = query
                .advance(context, &TeamStep::get_info_excluded(&id))
                .await?;

            let workset_infos = query
                .advance(context, &WorksetStep::list_by_team_id_excluded(&id))
                .await?;

            for workset in &workset_infos {
                query
                    .advance(context, &WorksetStep::delete_cascade(&workset.id))
                    .await?;
            }

            query.advance(context, &TeamStep::delete(&id)).await?;

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
