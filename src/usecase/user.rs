use poprako_transactional::advance::Advance;
use poprako_transactional::run::Run;
use poprako_util::i18n::trl;

use crate::data::user::{UserInfoUpdData, UserInfoVal};
use crate::model::user::UserToken;
use crate::part::image_pool::ImagePool;
use crate::part::query::action::member::MemberUpdUserNickname;
use crate::part::query::action::user::{UserGetInfoById, UserUpdInfo};
use crate::part::query::member::MemberQuery;
use crate::part::query::user::UserQuery;
use crate::part::query::{Execute, map_run_err};
use crate::result::{ExpectedVariant, RootError, RootResult, accept};

pub async fn get_info<H, Q, P>(
    query: Q,
    image_pool: P,
    token: UserToken,
    id: String,
) -> RootResult<UserInfoVal>
where
    Q: UserQuery<H>,
    P: ImagePool,
{
    // TODO: perm check.

    let info_model = Execute::execute(&query, UserGetInfoById { id: &id }).await?;

    accept(UserInfoVal::from_model(&image_pool, info_model).await)
}

pub async fn update_info<R, H, Q>(
    run: R,
    query: Q,
    token: UserToken,
    input: UserInfoUpdData,
) -> RootResult<()>
where
    R: Run<H>,
    R::Error: Into<RootError>,
    H: Send,
    Q: UserQuery<H> + MemberQuery<H> + Send,
    <Q as UserQuery<H>>::Transactional: Send,
    <Q as MemberQuery<H>>::Transactional: Send,
{
    if token.user_id != input.id {
        return Err(RootError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("no-permissiom"),
        });
    }

    let user_id = token.user_id;

    run.with_scope(async move |handle| {
        let mut user_query = UserQuery::transactional(&query);

        Advance::advance(
            &mut user_query,
            handle,
            UserUpdInfo {
                id: &user_id,
                qid: &input.qid,
                nickname: &input.nickname,
            },
        )
        .await?;

        let mut member_query = MemberQuery::transactional(&query);

        Advance::advance(
            &mut member_query,
            handle,
            MemberUpdUserNickname {
                user_id: &user_id,
                user_nickname: &input.nickname,
            },
        )
        .await?;

        accept(())
    })
    .await
    .map_err(map_run_err)
}
