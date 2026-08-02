//! Team-scoped online-user lease use cases.

use poprako_orchestra::run_proxy;
use tracing::instrument;

use crate::complex::team::{TeamComplex, TeamPermComplex};
use crate::model::shared::user::UserToken;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::result::{BaseRest, accept};

#[cfg(test)]
// Online-user use-case tests cover membership gates and team isolation.
mod tests;

/// Marks the authenticated user online in one team for ten minutes.
#[instrument(level = "info", err(Debug), skip(repo))]
pub async fn mark_self_online<C, R>(
    (repo,): (&R,),
    token: UserToken,
    team_id: String,
) -> BaseRest<()>
where
    R: MemberRepo<C> + Sync,
{
    TeamPermComplex::ensure_user_can_mark_self_online(
        &mut run_proxy! {
            repo => for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &team_id,
    )
    .await?;

    TeamComplex::mark_user_online(&team_id, &token.user_id);

    accept(())
}

/// Lists active user identifiers for one team in ascending order.
#[instrument(level = "info", err(Debug), skip(repo))]
pub async fn list_online_user_ids<C, R>(
    (repo,): (&R,),
    token: UserToken,
    team_id: String,
) -> BaseRest<Vec<String>>
where
    R: MemberRepo<C> + Sync,
{
    TeamPermComplex::ensure_user_can_list_online_user_ids(
        &mut run_proxy! {
            repo => for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &team_id,
    )
    .await?;

    let online_user_ids = TeamComplex::list_online_user_ids(&team_id);

    accept(online_user_ids)
}
