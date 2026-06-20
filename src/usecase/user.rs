use time::{Duration, OffsetDateTime};

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;
use poprako_util::i18n::trl;

use crate::complex::user::UserComplex;
use crate::data::user::{
    MarkUserAvatarUploadedData, ReserveUserAvatarData, ReserveUserAvatarVal, UpdateUserInfoData,
    UserInfoVal,
};
use crate::model::user::UserToken;
use crate::part::effect::event::Event;
use crate::part::effect::event::user::UserActivePayload;
use crate::part::effect::{Develop, EffectEmit as _};
use crate::part::image::ImagePool;
use crate::part::pledge::intention::{IMAGE_TOPIC, ImageIntention, ImageResourceKind};
use crate::part::pledge::{Payload, Prom, PromStep, PromTransactional};
use crate::part::query::member::{MemberQuery, MemberQueryTransactional};
use crate::part::query::step::member::MemberStep;
use crate::part::query::step::user::UserStep;
use crate::part::query::user::{UserQuery, UserQueryTransactional};
use crate::part::query::{DeriveTransactional, map_drive_err};
use crate::result::{ExpectedVariant, RootError, RootResult, accept};

pub async fn get_info<C, Q, P, V>(
    query: &Q,
    image_pool: &P,
    develop: &V,
    token: UserToken,
    id: String,
) -> RootResult<UserInfoVal>
where
    Q: UserQuery<C>,
    <Q as DeriveTransactional>::Transactional: UserQueryTransactional<C>,
    P: ImagePool,
    V: Develop + Send + Sync,
{
    let info_model = query.execute(UserStep::get_info_by_id(&id)).await?;

    if token.user_id == id {
        Event::UserActive(UserActivePayload {
            user_id: token.user_id,
        })
        .emit(develop)
        .await;
    }

    UserInfoVal::from_model(image_pool, info_model).await
}

pub async fn update_info<D, C, Q>(
    drive: &D,
    query: &Q,
    token: UserToken,
    data: UpdateUserInfoData,
) -> RootResult<()>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    Q: UserQuery<C> + MemberQuery<C> + Send + Sync,
    <Q as DeriveTransactional>::Transactional:
        UserQueryTransactional<C> + MemberQueryTransactional<C> + Send,
{
    if token.user_id != data.id {
        return Err(RootError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-forbidden"),
        });
    }

    drive
        .with_context(async move |context| {
            let query = DeriveTransactional::transactional(query).await;

            query
                .advance(
                    context,
                    UserStep::update_info(&token.user_id, &data.qid, &data.nickname),
                )
                .await?;

            query
                .advance(
                    context,
                    MemberStep::update_user_nickname(&token.user_id, &data.nickname),
                )
                .await?;

            accept(())
        })
        .await
        .map_err(map_drive_err)
}

pub async fn reserve_avatar<D, C, Q, P, I>(
    drive: &D,
    query: &Q,
    pledge: &P,
    image: &I,
    token: UserToken,
    data: ReserveUserAvatarData,
) -> RootResult<ReserveUserAvatarVal>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    Q: UserQuery<C> + Send + Sync,
    <Q as DeriveTransactional>::Transactional: UserQueryTransactional<C> + Send,
    P: Prom<C> + Send + Sync,
    <P as DeriveTransactional>::Transactional: PromTransactional<C> + Send + Sync,
    I: ImagePool,
{
    let (object_key, avatar_version) = drive
        .with_context(async move |context| {
            let query = DeriveTransactional::transactional(query).await;
            let prom = DeriveTransactional::transactional(pledge).await;

            let reservation = query
                .advance(
                    context,
                    UserStep::reserve_avatar(&token.user_id, &data.file_ext),
                )
                .await?;

            let now = OffsetDateTime::now_utc();

            if let Some(previous_key) = &reservation.previous_object_key {
                let delete_id = UserComplex::generate_avatar_delete_id();

                prom.advance(
                    context,
                    PromStep::append(
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

            let check_id = UserComplex::generate_avatar_check_id();
            let check_visible_at = now + Duration::minutes(15);

            prom.advance(
                context,
                PromStep::append(
                    &check_id,
                    IMAGE_TOPIC,
                    Payload::Image(ImageIntention::CheckUploaded {
                        resource_kind: ImageResourceKind::UserAvatar,
                        resource_id: token.user_id.clone(),
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

    Ok(ReserveUserAvatarVal {
        put_url,
        avatar_version,
    })
}

pub async fn mark_avatar_uploaded<D, C, Q>(
    drive: &D,
    query: &Q,
    token: UserToken,
    id: String,
    data: MarkUserAvatarUploadedData,
) -> RootResult<()>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    Q: UserQuery<C> + Send + Sync,
    <Q as DeriveTransactional>::Transactional: UserQueryTransactional<C> + Send,
{
    if token.user_id != id {
        return Err(RootError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-forbidden"),
        });
    }

    drive
        .with_context(async move |context| {
            let query = DeriveTransactional::transactional(query).await;

            query
                .advance(
                    context,
                    UserStep::mark_avatar_uploaded(&id, data.avatar_version),
                )
                .await?;

            accept(())
        })
        .await
        .map_err(map_drive_err)
}

pub async fn touch_last_active<D, C, Q>(drive: &D, query: &Q, token: UserToken) -> RootResult<()>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    Q: UserQuery<C> + MemberQuery<C> + Send + Sync,
    <Q as DeriveTransactional>::Transactional:
        UserQueryTransactional<C> + MemberQueryTransactional<C> + Send,
{
    drive
        .with_context(async move |context| {
            let query = DeriveTransactional::transactional(query).await;

            query
                .advance(context, UserStep::touch_last_active(&token.user_id))
                .await?;

            query
                .advance(context, MemberStep::touch_last_active(&token.user_id))
                .await?;

            accept(())
        })
        .await
        .map_err(map_drive_err)
}

pub async fn delete<D, C, Q, P>(
    drive: &D,
    query: &Q,
    pledge: &P,
    token: UserToken,
    id: String,
) -> RootResult<()>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    Q: UserQuery<C> + MemberQuery<C> + Send + Sync,
    <Q as DeriveTransactional>::Transactional:
        UserQueryTransactional<C> + MemberQueryTransactional<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
    <P as DeriveTransactional>::Transactional: PromTransactional<C> + Send + Sync,
{
    if token.user_id != id {
        // TODO: perm check.
        return Err(RootError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-forbidden"),
        });
    }

    drive
        .with_context(async move |context| {
            let query = DeriveTransactional::transactional(query).await;
            let prom = DeriveTransactional::transactional(pledge).await;

            let user_info = query
                .advance(context, UserStep::get_info_excluded(&id))
                .await?;

            let member_infos = query
                .advance(context, MemberStep::list_by_user_id_excluded(&id))
                .await?;

            for mi in &member_infos {
                query.advance(context, MemberStep::delete(&mi.id)).await?;
            }

            query.advance(context, UserStep::delete(&id)).await?;

            if let Some(avatar_key) = &user_info.avatar_key
                && user_info.avatar_uploaded
            {
                let now = OffsetDateTime::now_utc();
                let delete_id = UserComplex::generate_avatar_delete_id();

                prom.advance(
                    context,
                    PromStep::append(
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
