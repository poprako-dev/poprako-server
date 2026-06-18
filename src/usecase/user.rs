use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;
use poprako_util::i18n::trl;
use time::Duration;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::data::user::{
    UserAvatarMarkUploadedData, UserAvatarReserveData, UserAvatarReserveVal, UserInfoUpdateData,
    UserInfoVal,
};
use crate::model::local_message::{IMAGE_TOPIC, ImageLocalMessage, ImageResourceKind};
use crate::model::user::{UserInfoUpdate, UserToken};
use crate::part::effect::event::Event;
use crate::part::effect::event::user::UserActivePayload;
use crate::part::effect::{Develop, EffectEmit as _};
use crate::part::image_pool::ImagePool;
use crate::part::pledge::{Append, Payload, Pledge};
use crate::part::query::member::{MemberQuery, MemberQueryTransactional};
use crate::part::query::step::member::{
    MemberDelete, MemberListByUserIdExcluded, MemberTouchLastActive, MemberUpdateUserNickname,
};
use crate::part::query::step::user::{
    UserDelete, UserGetInfoById, UserGetInfoExcluded, UserMarkAvatarUploaded, UserReserveAvatar,
    UserTouchLastActive, UserUpdateInfo,
};
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
    let info_model = Execute::execute(&query, UserGetInfoById { id: &id }).await?;

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
    input: UserInfoUpdateData,
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
                    UserUpdateInfo {
                        input: UserInfoUpdate {
                            id: &token.user_id,
                            qid: &input.qid,
                            nickname: &input.nickname,
                        },
                    },
                )
                .await?;

            query
                .advance(
                    handle,
                    MemberUpdateUserNickname {
                        user_id: &token.user_id,
                        user_nickname: &input.nickname,
                    },
                )
                .await?;

            accept(())
        })
        .await
        .map_err(map_drive_err)
}

pub async fn reserve_avatar<D, H, Q, P>(
    drive: D,
    query: Q,
    image_pool: P,
    token: UserToken,
    user_id: String,
    input: UserAvatarReserveData,
) -> RootResult<UserAvatarReserveVal>
where
    D: Drive<H>,
    D::Error: Into<RootError>,
    H: Send,
    Q: UserQuery<H> + Send,
    <Q as DeriveTransactional>::Transactional: UserQueryTransactional<H> + Pledge<H> + Send,
    P: ImagePool,
{
    if token.user_id != user_id {
        return Err(RootError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-forbidden"),
        });
    }

    let result: (String, i64) = drive
        .run_transactional(async move |handle| {
            let mut query = DeriveTransactional::transactional(&query).await;

            let reservation = query
                .advance(
                    handle,
                    UserReserveAvatar {
                        id: &user_id,
                        file_ext: &input.file_ext,
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
                            resource_kind: ImageResourceKind::UserAvatar,
                            resource_id: user_id.clone(),
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

    let put_url = image_pool.put_signed(&object_key).await?.to_string();

    Ok(UserAvatarReserveVal {
        put_url,
        avatar_version,
    })
}

pub async fn mark_avatar_uploaded<D, H, Q>(
    drive: D,
    query: Q,
    token: UserToken,
    user_id: String,
    input: UserAvatarMarkUploadedData,
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
                    UserMarkAvatarUploaded {
                        id: &user_id,
                        avatar_version: input.avatar_version,
                    },
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
                .advance(handle, UserTouchLastActive { id: &token.user_id })
                .await?;

            query
                .advance(
                    handle,
                    MemberTouchLastActive {
                        user_id: &token.user_id,
                    },
                )
                .await?;

            accept(())
        })
        .await
        .map_err(map_drive_err)
}

pub async fn delete_user<D, H, Q>(
    drive: D,
    query: Q,
    token: UserToken,
    user_id: String,
) -> RootResult<()>
where
    D: Drive<H>,
    D::Error: Into<RootError>,
    H: Send,
    Q: UserQuery<H> + MemberQuery<H> + Send,
    <Q as DeriveTransactional>::Transactional:
        UserQueryTransactional<H> + MemberQueryTransactional<H> + Pledge<H> + Send,
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

            let user_info = query
                .advance(handle, UserGetInfoExcluded { id: &user_id })
                .await?;

            let members = query
                .advance(handle, MemberListByUserIdExcluded { user_id: &user_id })
                .await?;

            for member in &members {
                query
                    .advance(handle, MemberDelete { id: &member.id })
                    .await?;
            }

            query.advance(handle, UserDelete { id: &user_id }).await?;

            if let Some(ref avatar_key) = user_info.avatar_key {
                if user_info.avatar_uploaded {
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
