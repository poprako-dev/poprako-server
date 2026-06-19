use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;
use poprako_util::i18n::trl;

use crate::data::user::{
    MarkUserAvatarUploadedData, ReserveUserAvatarData, ReserveUserAvatarVal, UpdateUserInfoData,
    UserInfoVal,
};
use crate::model::user::UserToken;
use crate::part::effect::event::Event;
use crate::part::effect::event::user::UserActivePayload;
use crate::part::effect::{Develop, EffectEmit as _};
use crate::part::image_pool::ImagePool;
use crate::part::intention::{IMAGE_TOPIC, ImageLocalMessage, ImageResourceKind};
use crate::part::pledge::{Payload, Pledge, PledgeStep};
use crate::part::query::member::{MemberQuery, MemberQueryTransactional};
use crate::part::query::step::member::MemberStep;
use crate::part::query::step::user::UserStep;
use crate::part::query::user::{UserQuery, UserQueryTransactional};
use crate::part::query::{DeriveTransactional, Execute, map_drive_err};
use crate::result::{ExpectedVariant, RootError, RootResult, accept};

pub async fn get_info<H, Q, P, D>(
    query: Q,
    image_pool: P,
    develop: &D,
    token: UserToken,
    id: String,
) -> RootResult<UserInfoVal>
where
    Q: UserQuery<H>,
    <Q as DeriveTransactional>::Transactional: UserQueryTransactional<H>,
    P: ImagePool,
    D: Develop + Send + Sync,
{
    let info_model = Execute::execute(&query, UserStep::get_info_by_id(&id)).await?;

    if token.user_id == id {
        Event::UserActive(UserActivePayload {
            user_id: token.user_id,
        })
        .emit(develop)
        .await;
    }

    UserInfoVal::from_model(&image_pool, info_model).await
}

pub async fn update_info<D, H, Q>(
    drive: D,
    query: Q,
    token: UserToken,
    input: UpdateUserInfoData,
) -> RootResult<()>
where
    D: Drive<H>,
    D::Error: Into<RootError>,
    H: Send,
    Q: UserQuery<H> + MemberQuery<H> + Send,
    <Q as DeriveTransactional>::Transactional:
        UserQueryTransactional<H> + MemberQueryTransactional<H> + Send,
{
    if token.user_id != input.id {
        return Err(RootError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-forbidden"),
        });
    }

    drive
        .run_transactional(async move |handle| {
            let mut query = DeriveTransactional::transactional(&query).await;

            query
                .advance(
                    handle,
                    UserStep::update_info(&token.user_id, &input.qid, &input.nickname),
                )
                .await?;

            query
                .advance(
                    handle,
                    MemberStep::update_user_nickname(&token.user_id, &input.nickname),
                )
                .await?;

            accept(())
        })
        .await
        .map_err(map_drive_err)
}

pub async fn reserve_avatar<D, H, Q, PL, P>(
    drive: D,
    query: Q,
    pledge: PL,
    image_pool: P,
    token: UserToken,
    user_id: String,
    input: ReserveUserAvatarData,
) -> RootResult<ReserveUserAvatarVal>
where
    D: Drive<H>,
    D::Error: Into<RootError>,
    H: Send,
    Q: UserQuery<H> + Send,
    <Q as DeriveTransactional>::Transactional: UserQueryTransactional<H> + Send,
    PL: Pledge<H> + Send,
    P: ImagePool,
{
    if token.user_id != user_id {
        return Err(RootError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-forbidden"),
        });
    }

    let (object_key, avatar_version) = drive
        .run_transactional(async move |handle| {
            let mut query = DeriveTransactional::transactional(&query).await;
            let mut pledge = pledge;

            let reservation = query
                .advance(handle, UserStep::reserve_avatar(&user_id, &input.file_ext))
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
                            resource_kind: ImageResourceKind::UserAvatar,
                            resource_id: user_id.clone(),
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

    Ok(ReserveUserAvatarVal {
        put_url,
        avatar_version,
    })
}

pub async fn mark_avatar_uploaded<D, H, Q>(
    drive: D,
    query: Q,
    token: UserToken,
    user_id: String,
    input: MarkUserAvatarUploadedData,
) -> RootResult<()>
where
    D: Drive<H>,
    D::Error: Into<RootError>,
    H: Send,
    Q: UserQuery<H> + Send,
    <Q as DeriveTransactional>::Transactional: UserQueryTransactional<H> + Send,
{
    if token.user_id != user_id {
        return Err(RootError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-forbidden"),
        });
    }

    drive
        .run_transactional(async move |handle| {
            let mut query = DeriveTransactional::transactional(&query).await;

            query
                .advance(
                    handle,
                    UserStep::mark_avatar_uploaded(&user_id, input.avatar_version),
                )
                .await?;

            accept(())
        })
        .await
        .map_err(map_drive_err)
}

pub async fn touch_last_active<D, H, Q>(drive: D, query: Q, token: UserToken) -> RootResult<()>
where
    D: Drive<H>,
    D::Error: Into<RootError>,
    H: Send,
    Q: UserQuery<H> + MemberQuery<H> + Send,
    <Q as DeriveTransactional>::Transactional:
        UserQueryTransactional<H> + MemberQueryTransactional<H> + Send,
{
    drive
        .run_transactional(async move |handle| {
            let mut query = DeriveTransactional::transactional(&query).await;

            query
                .advance(handle, UserStep::touch_last_active(&token.user_id))
                .await?;

            query
                .advance(handle, MemberStep::touch_last_active(&token.user_id))
                .await?;

            accept(())
        })
        .await
        .map_err(map_drive_err)
}

pub async fn delete_user<D, H, Q, PL>(
    drive: D,
    query: Q,
    pledge: PL,
    token: UserToken,
    user_id: String,
) -> RootResult<()>
where
    D: Drive<H>,
    D::Error: Into<RootError>,
    H: Send,
    Q: UserQuery<H> + MemberQuery<H> + Send,
    <Q as DeriveTransactional>::Transactional:
        UserQueryTransactional<H> + MemberQueryTransactional<H> + Send,
    PL: Pledge<H> + Send,
{
    if token.user_id != user_id {
        return Err(RootError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-forbidden"),
        });
    }

    drive
        .run_transactional(async move |handle| {
            let mut query = DeriveTransactional::transactional(&query).await;
            let mut pledge = pledge;

            let user_info = query
                .advance(handle, UserStep::get_info_excluded(&user_id))
                .await?;

            let members = query
                .advance(handle, MemberStep::list_by_user_id_excluded(&user_id))
                .await?;

            for member in &members {
                query
                    .advance(handle, MemberStep::delete(&member.id))
                    .await?;
            }

            query.advance(handle, UserStep::delete(&user_id)).await?;

            if let Some(avatar_key) = &user_info.avatar_key
                && user_info.avatar_uploaded
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
