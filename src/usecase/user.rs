use poprako_transactional::advance::Advance;
use poprako_transactional::run::Run;
use poprako_util::i18n::trl;

use crate::data::user::{UpdateInfoInput, UserToken};
use crate::part::query::member::MemberQuery;
use crate::part::query::user::UserQuery;
use crate::part::query::{action, map_run_err};
use crate::result::{ExpectedVariant, RootError, RootResult, accept};

pub async fn update_info<R, H, Q>(
    run: R,
    query: Q,
    token: UserToken,
    input: UpdateInfoInput,
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
        let user_update_info = action::user::UpdateInfo {
            id: &user_id,
            qid: &input.qid,
            nickname: &input.nickname,
        };

        let mut user_query = UserQuery::transactional(&query);

        Advance::advance(&mut user_query, handle, user_update_info).await?;

        let member_update_user_nickname = action::member::UpdateUserNickname {
            user_id: &user_id,
            user_nickname: &input.nickname,
        };

        let mut member_query = MemberQuery::transactional(&query);

        Advance::advance(&mut member_query, handle, member_update_user_nickname).await?;

        accept(())
    })
    .await
    .map_err(map_run_err)
}
