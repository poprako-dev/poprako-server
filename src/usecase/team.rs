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
use crate::part::image_pool::ImagePool;
use crate::part::intention::{IMAGE_TOPIC, ImageLocalMessage, ImageResourceKind};
use crate::part::pledge::{Payload, Pledge, PledgeStep};
use crate::part::query::step::team::TeamStep;
use crate::part::query::step::workset::WorksetStep;
use crate::part::query::team::{TeamQuery, TeamQueryTransactional};
use crate::part::query::workset::{WorksetQuery, WorksetQueryTransactional};
use crate::part::query::{DeriveTransactional, Execute, map_drive_err};
use crate::result::{RootError, RootResult, accept};

pub async fn create<H, Q, P>(
    query: Q,
    image_pool: P,
    data: CreateTeamData,
) -> RootResult<TeamInfoVal>
where
    Q: TeamQuery<H>,
    <Q as DeriveTransactional>::Transactional: TeamQueryTransactional<H>,
    P: ImagePool,
{
    let team_form = TeamForm {
        id: format!("team-{}", Uuid::now_v7()),
        name: data.name,
        description: data.description,
    };

    let team_info = Execute::execute(&query, TeamStep::create(&team_form)).await?;

    TeamInfoVal::from_model(&image_pool, team_info).await
}

pub async fn get_info<H, Q, P>(query: Q, image_pool: P, team_id: String) -> RootResult<TeamInfoVal>
where
    Q: TeamQuery<H>,
    <Q as DeriveTransactional>::Transactional: TeamQueryTransactional<H>,
    P: ImagePool,
{
    let team_info = Execute::execute(&query, TeamStep::get_info_by_id(&team_id)).await?;

    TeamInfoVal::from_model(&image_pool, team_info).await
}

pub async fn list_infos<H, Q, P>(
    query: Q,
    image_pool: P,
    page: Page,
) -> RootResult<Vec<TeamInfoVal>>
where
    Q: TeamQuery<H>,
    <Q as DeriveTransactional>::Transactional: TeamQueryTransactional<H>,
    P: ImagePool,
{
    let team_infos = Execute::execute(&query, TeamStep::list(page)).await?;

    let mut vals = Vec::with_capacity(team_infos.len());
    for info in team_infos {
        vals.push(TeamInfoVal::from_model(&image_pool, info).await?);
    }

    Ok(vals)
}

pub async fn update_info<H, Q>(query: Q, input: UpdateTeamInfoData) -> RootResult<()>
where
    Q: TeamQuery<H>,
    <Q as DeriveTransactional>::Transactional: TeamQueryTransactional<H>,
{
    Execute::execute(
        &query,
        TeamStep::update_info(&input.id, &input.name, &input.description),
    )
    .await?;

    Ok(())
}

pub async fn reserve_avatar<D, H, Q, Pl, P>(
    drive: D,
    query: Q,
    pledge: Pl,
    image_pool: P,
    team_id: String,
    input: ReserveTeamAvatarData,
) -> RootResult<ReserveTeamAvatarVal>
where
    D: Drive<H>,
    D::Error: Into<RootError>,
    H: Send,
    Q: TeamQuery<H> + Send,
    <Q as DeriveTransactional>::Transactional: TeamQueryTransactional<H> + Send,
    Pl: Pledge<H> + Send,
    P: ImagePool,
{
    let (object_key, avatar_version) = drive
        .run_transactional(async move |handle| {
            let mut query = DeriveTransactional::transactional(&query).await;
            let mut pledge = pledge;

            let reservation = query
                .advance(handle, TeamStep::reserve_avatar(&team_id, &input.file_ext))
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
                            Payload::Image(ImageLocalMessage::Delete {
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
                        Payload::Image(ImageLocalMessage::CheckUploaded {
                            resource_kind: ImageResourceKind::TeamAvatar,
                            resource_id: team_id.clone(),
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

    let put_url = image_pool.put_signed(&object_key).await?.to_string();

    Ok(ReserveTeamAvatarVal {
        put_url,
        avatar_version,
    })
}

pub async fn mark_avatar_uploaded<H, Q>(
    query: Q,
    team_id: String,
    input: MarkTeamAvatarUploadedData,
) -> RootResult<()>
where
    Q: TeamQuery<H>,
    <Q as DeriveTransactional>::Transactional: TeamQueryTransactional<H>,
{
    Execute::execute(
        &query,
        TeamStep::mark_avatar_uploaded(&team_id, input.avatar_version),
    )
    .await?;

    Ok(())
}

pub async fn delete<D, H, Q, Pl>(drive: D, query: Q, pledge: Pl, team_id: String) -> RootResult<()>
where
    D: Drive<H>,
    D::Error: Into<RootError>,
    H: Send,
    Q: TeamQuery<H> + WorksetQuery<H> + Send,
    <Q as DeriveTransactional>::Transactional:
        TeamQueryTransactional<H> + WorksetQueryTransactional<H> + Send,
    Pl: Pledge<H> + Send,
{
    drive
        .run_transactional(async move |handle| {
            let mut query = DeriveTransactional::transactional(&query).await;
            let mut pledge = pledge;

            let team_info = query
                .advance(handle, TeamStep::get_info_excluded(&team_id))
                .await?;

            let worksets = query
                .advance(handle, WorksetStep::list_by_team_id_excluded(&team_id))
                .await?;

            for workset in &worksets {
                query
                    .advance(handle, WorksetStep::delete_cascade(&workset.id))
                    .await?;
            }

            query.advance(handle, TeamStep::delete(&team_id)).await?;

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
                            Payload::Image(ImageLocalMessage::Delete {
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
