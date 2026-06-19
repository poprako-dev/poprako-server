use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;
use poprako_util::page::Page;

use crate::data::team::{
    CreateTeamData, MarkTeamAvatarUploadedData, ReserveTeamAvatarData, ReserveTeamAvatarVal,
    TeamInfoVal, UpdateTeamInfoData,
};
use crate::model::team::TeamForm;
use crate::part::image::ImagePool;
use crate::part::pledge::intention::{IMAGE_TOPIC, ImageIntention, ImageResourceKind};
use crate::part::pledge::{Payload, Pledge, PledgeStep};
use crate::part::query::step::team::TeamStep;
use crate::part::query::step::workset::WorksetStep;
use crate::part::query::team::{TeamQuery, TeamQueryTransactional};
use crate::part::query::workset::{WorksetQuery, WorksetQueryTransactional};
use crate::part::query::{DeriveTransactional, Execute, map_drive_err};
use crate::result::{RootError, RootResult, accept};

pub async fn create<H, Q, I>(query: Q, image: I, data: CreateTeamData) -> RootResult<TeamInfoVal>
where
    Q: TeamQuery<H>,
    <Q as DeriveTransactional>::Transactional: TeamQueryTransactional<H>,
    I: ImagePool,
{
    let form = TeamForm {
        id: format!("team-{}", Uuid::now_v7()),
        name: data.name,
        description: data.description,
    };

    let info = query.execute(TeamStep::create(&form)).await?;

    TeamInfoVal::from_model(&image, info).await
}

pub async fn get_info<H, Q, I>(query: Q, image: I, id: String) -> RootResult<TeamInfoVal>
where
    Q: TeamQuery<H>,
    <Q as DeriveTransactional>::Transactional: TeamQueryTransactional<H>,
    I: ImagePool,
{
    let info = query.execute(TeamStep::get_info_by_id(&id)).await?;

    TeamInfoVal::from_model(&image, info).await
}

pub async fn list_infos<H, Q, I>(query: Q, image: I, page: Page) -> RootResult<Vec<TeamInfoVal>>
where
    Q: TeamQuery<H>,
    <Q as DeriveTransactional>::Transactional: TeamQueryTransactional<H>,
    I: ImagePool,
{
    let infos = query.execute(TeamStep::list(page)).await?;

    // TODO: join all.
    let mut vals = Vec::with_capacity(infos.len());
    for info in infos {
        vals.push(TeamInfoVal::from_model(&image, info).await?);
    }

    Ok(vals)
}

pub async fn update_info<H, Q>(query: Q, data: UpdateTeamInfoData) -> RootResult<()>
where
    Q: TeamQuery<H>,
    <Q as DeriveTransactional>::Transactional: TeamQueryTransactional<H>,
{
    query
        .execute(TeamStep::update_info(
            &data.id,
            &data.name,
            &data.description,
        ))
        .await?;

    Ok(())
}

pub async fn reserve_avatar<D, H, Q, P, I>(
    drive: D,
    query: Q,
    pledge: P,
    image: I,
    id: String,
    input: ReserveTeamAvatarData,
) -> RootResult<ReserveTeamAvatarVal>
where
    D: Drive<H>,
    D::Error: Into<RootError>,
    H: Send,
    Q: TeamQuery<H> + Send,
    <Q as DeriveTransactional>::Transactional: TeamQueryTransactional<H> + Send,
    P: Pledge<H> + Send,
    I: ImagePool,
{
    let (object_key, avatar_version) = drive
        .run_transactional(async move |handle| {
            let mut query = DeriveTransactional::transactional(&query).await;

            let reservation = query
                .advance(handle, TeamStep::reserve_avatar(&id, &input.file_ext))
                .await?;

            let now = OffsetDateTime::now_utc();

            if let Some(previous_key) = &reservation.previous_object_key {
                let delete_id = format!("lm-{}", Uuid::now_v7());

                pledge
                    .advance(
                        handle,
                        PledgeStep::append(
                            &delete_id,
                            IMAGE_TOPIC.to_string(),
                            Payload::Image(ImageIntention::Delete {
                                object_key: previous_key.clone(),
                            }),
                            &now,
                        ),
                    )
                    .await?;
            }

            let check_id = format!("lm-{}", Uuid::now_v7());
            let check_visible_at = now + Duration::minutes(15);

            pledge
                .advance(
                    handle,
                    PledgeStep::append(
                        &check_id,
                        IMAGE_TOPIC.to_string(),
                        Payload::Image(ImageIntention::CheckUploaded {
                            resource_kind: ImageResourceKind::TeamAvatar,
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

pub async fn mark_avatar_uploaded<H, Q>(
    query: Q,
    id: String,
    data: MarkTeamAvatarUploadedData,
) -> RootResult<()>
where
    Q: TeamQuery<H>,
    <Q as DeriveTransactional>::Transactional: TeamQueryTransactional<H>,
{
    query
        .execute(TeamStep::mark_avatar_uploaded(&id, data.avatar_version))
        .await?;

    Ok(())
}

pub async fn delete<D, H, Q, P>(drive: D, query: Q, pledge: P, id: String) -> RootResult<()>
where
    D: Drive<H>,
    D::Error: Into<RootError>,
    H: Send,
    Q: TeamQuery<H> + WorksetQuery<H> + Send,
    <Q as DeriveTransactional>::Transactional:
        TeamQueryTransactional<H> + WorksetQueryTransactional<H> + Send,
    P: Pledge<H> + Send,
{
    drive
        .run_transactional(async move |handle| {
            let mut query = DeriveTransactional::transactional(&query).await;
            let mut pledge = pledge;

            let team_info = query
                .advance(handle, TeamStep::get_info_excluded(&id))
                .await?;

            let worksets = query
                .advance(handle, WorksetStep::list_by_team_id_excluded(&id))
                .await?;

            for workset in &worksets {
                query
                    .advance(handle, WorksetStep::delete_cascade(&workset.id))
                    .await?;
            }

            query.advance(handle, TeamStep::delete(&id)).await?;

            if let Some(avatar_key) = &team_info.avatar_key
                && team_info.avatar_uploaded
            {
                let now = OffsetDateTime::now_utc();
                let delete_id = format!("lm-{}", Uuid::now_v7());
                pledge
                    .advance(
                        handle,
                        PledgeStep::append(
                            &delete_id,
                            IMAGE_TOPIC.to_string(),
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
