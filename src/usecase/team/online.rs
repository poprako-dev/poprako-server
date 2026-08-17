//! Team-scoped online-user lease use cases.

#[cfg(test)]
// Online-user use-case tests cover membership gates and team isolation.
mod tests;

use poprako_orchestra::{Context, OperRun as _, run_proxy};
use tracing::instrument;

use crate::complex::team::TeamPermComplex;
use crate::model::shared::user::UserToken;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::online_user::OnlineUserRepo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::online_user::{ListOnlineUserIds, MarkOnlineUser};
use crate::result::{BaseRest, accept};

/// Marks the authenticated user online in one team for ten minutes.
#[instrument(level = "info", skip(repo))]
pub async fn mark_self_online<C, R>(
    (repo,): (&R,),
    token: UserToken,
    team_id: String,
) -> BaseRest<()>
where
    C: Context,
    R: MemberRepo<C> + OnlineUserRepo + Sync,
{
    TeamPermComplex::ensure_user_can_mark_self_online(
        &mut run_proxy! {
            repo => for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &team_id,
    )
    .await?;

    MarkOnlineUser {
        team_id: &team_id,
        user_id: &token.user_id,
    }
    .run_on(repo)
    .await?;

    accept(())
}

/// Lists active user identifiers for one team in ascending order.
#[instrument(level = "info", skip(repo))]
pub async fn list_online_user_ids<C, R>(
    (repo,): (&R,),
    token: UserToken,
    team_id: String,
) -> BaseRest<Vec<String>>
where
    C: Context,
    R: MemberRepo<C> + OnlineUserRepo + Sync,
{
    TeamPermComplex::ensure_user_can_list_online_user_ids(
        &mut run_proxy! {
            repo => for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &team_id,
    )
    .await?;

    let online_user_ids =
        ListOnlineUserIds { team_id: &team_id }.run_on(repo).await?;

    accept(online_user_ids)
}
