use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;
use poprako_util::page::Page;
use time::Duration;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::data::team::{
    TeamAvatarMarkUploadedData,
    TeamAvatarReserveData,
    TeamAvatarReserveVal,
    TeamCreateData,
    TeamInfoUpdateData,
    TeamInfoVal,
};
use crate::model::local_message::{ImageLocalMessage, ImageResourceKind, IMAGE_TOPIC};
use crate::model::team::{TeamForm, TeamInfoUpdate};
use crate::part::image_pool::ImagePool;
use crate::part::pledge::{Append, Payload, Pledge};
use crate::part::query::step::team::{
    TeamCreate,
    TeamDelete,
    TeamGetInfoById,
    TeamGetInfoExcluded,
    TeamList,
    TeamMarkAvatarUploaded,
    TeamReserveAvatar,
    TeamUpdateInfo,
};
use crate::part::query::step::workset::{WorksetDeleteCascade, WorksetListByTeamIdExcluded};
use crate::part::query::team::{TeamQuery, TeamQueryTransactional};
use crate::part::query::workset::{WorksetQuery, WorksetQueryTransactional};
use crate::part::query::{DeriveTransactional, Execute, map_drive_err};
use crate::result::{RootError, RootResult, accept};

pub async fn create<H, Q, P>(
    query: Q,
    image_pool: P,
    input: TeamCreateData,
) -> RootResult<TeamInfoVal>
where
    Q: TeamQuery<H>,
    <Q as DeriveTransactional>::Transactional: TeamQueryTransactional<H>,
    P: ImagePool,
{
    let team_form = TeamForm {
        id: format!("team-{}", Uuid::now_v7()),
        name: input.name,
        description: input.description,
    };

    let team_info = Execute::execute(&query, TeamCreate { form: &team_form }).await?;

    TeamInfoVal::from_model(&image_pool, team_info).await
}

pub async fn get_info<H, Q, P>(
    query: Q,
    image_pool: P,
    team_id: String,
) -> RootResult<TeamInfoVal>
where
    Q: TeamQuery<H>,
    <Q as DeriveTransactional>::Transactional: TeamQueryTransactional<H>,
    P: ImagePool,
{
    let team_info = Execute::execute(&query, TeamGetInfoById { id: &team_id }).await?;

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
    let team_infos = Execute::execute(&query, TeamList { page }).await?;

    let mut vals = Vec::with_capacity(team_infos.len());
    for info in team_infos {
        vals.push(TeamInfoVal::from_model(&image_pool, info).await?);
    }
    Ok(vals)
}

pub async fn update_info<H, Q>(
    query: Q,
    input: TeamInfoUpdateData,
) -> RootResult<()>
where
    Q: TeamQuery<H>,
    <Q as DeriveTransactional>::Transactional: TeamQueryTransactional<H>,
{
    Execute::execute(
        &query,
        TeamUpdateInfo {
            input: TeamInfoUpdate {
                id: &input.id,
                name: &input.name,
                description: &input.description,
            },
        },
    )
    .await?;

    Ok(())
}

pub async fn reserve_avatar<D, H, Q, P>(
    drive: D,
    query: Q,
    image_pool: P,
    team_id: String,
    input: TeamAvatarReserveData,
) -> RootResult<TeamAvatarReserveVal>
where
    D: Drive<H>,
    D::Error: Into<RootError>,
    H: Send,
    Q: TeamQuery<H> + Send,
    <Q as DeriveTransactional>::Transactional: TeamQueryTransactional<H> + Pledge<H> + Send,
    P: ImagePool,
{
    let result: (String, i64) = drive
        .run_transactional(async move |handle| {
            let mut query = DeriveTransactional::transactional(&query).await;

            let reservation = query
                .advance(
                    handle,
                    TeamReserveAvatar {
                        id: &team_id,
                        file_extension: &input.file_ext,
                    },
                )
                .await?;

            let now = OffsetDateTime::now_utc();

            if let Some(ref previous_key) = reservation.previous_object_key {
                let delete_id = format!("lm-{}", Uuid::now_v7());
                query
                    .advance(
                        handle,
                        Append {
                            id: &delete_id,
                            topic: IMAGE_TOPIC.to_string(),
                            payload: Payload::Image(ImageLocalMessage::Delete {
                                object_key: previous_key.clone(),
                            }),
                            visible_at: &now,
                        },
                    )
                    .await?;
            }

            let check_id = format!("lm-{}", Uuid::now_v7());
            let check_visible_at = now + Duration::minutes(15);
            query
                .advance(
                    handle,
                    Append {
                        id: &check_id,
                        topic: IMAGE_TOPIC.to_string(),
                        payload: Payload::Image(ImageLocalMessage::CheckUploaded {
                            resource_kind: ImageResourceKind::TeamAvatar,
                            resource_id: team_id.clone(),
                            object_key: reservation.object_key.clone(),
                            image_version: reservation.avatar_version,
                        }),
                        visible_at: &check_visible_at,
                    },
                )
                .await?;

            accept((reservation.object_key, reservation.avatar_version))
        })
        .await
        .map_err(map_drive_err)?;

    let object_key = result.0;
    let avatar_version = result.1;

    let put_url = image_pool
        .put_signed(&object_key)
        .await?
        .to_string();

    Ok(TeamAvatarReserveVal {
        put_url,
        avatar_version,
    })
}

pub async fn mark_avatar_uploaded<H, Q>(
    query: Q,
    team_id: String,
    input: TeamAvatarMarkUploadedData,
) -> RootResult<()>
where
    Q: TeamQuery<H>,
    <Q as DeriveTransactional>::Transactional: TeamQueryTransactional<H>,
{
    Execute::execute(
        &query,
        TeamMarkAvatarUploaded {
            id: &team_id,
            avatar_version: input.avatar_version,
        },
    )
    .await?;

    Ok(())
}

pub async fn delete<D, H, Q>(
    drive: D,
    query: Q,
    team_id: String,
) -> RootResult<()>
where
    D: Drive<H>,
    D::Error: Into<RootError>,
    H: Send,
    Q: TeamQuery<H> + WorksetQuery<H> + Send,
    <Q as DeriveTransactional>::Transactional:
        TeamQueryTransactional<H> + WorksetQueryTransactional<H> + Pledge<H> + Send,
{
    drive
        .run_transactional(async move |handle| {
            let mut query = DeriveTransactional::transactional(&query).await;

            let team_info = query
                .advance(
                    handle,
                    TeamGetInfoExcluded { id: &team_id },
                )
                .await?;

            let worksets = query
                .advance(
                    handle,
                    WorksetListByTeamIdExcluded {
                        team_id: &team_id,
                    },
                )
                .await?;

            for workset in &worksets {
                query
                    .advance(handle, WorksetDeleteCascade { id: &workset.id })
                    .await?;
            }

            query
                .advance(handle, TeamDelete { id: &team_id })
                .await?;

            if let Some(ref avatar_key) = team_info.avatar_key {
                if team_info.avatar_uploaded {
                    let now = OffsetDateTime::now_utc();
                    let delete_id = format!("lm-{}", Uuid::now_v7());
                    query
                        .advance(
                            handle,
                            Append {
                                id: &delete_id,
                                topic: IMAGE_TOPIC.to_string(),
                                payload: Payload::Image(ImageLocalMessage::Delete {
                                    object_key: avatar_key.clone(),
                                }),
                                visible_at: &now,
                            },
                        )
                        .await?;
                }
            }

            accept(())
        })
        .await
        .map_err(map_drive_err)
}
