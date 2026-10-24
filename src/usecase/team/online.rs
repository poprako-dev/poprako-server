//! Team-scoped online-user lease use cases.

#[cfg(test)]
// Online-user use-case tests cover membership gates and team isolation.
mod tests;

use poprako_orchestra::{Context, OperRun as _};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::team::perm as team_perm_complex;
use crate::model::shared::user::UserToken;
use crate::part::cache::online_user::OnlineUserCache;
use crate::part::cache::oper::online_user::{
    ListOnlineUserIds, MarkUserOnline,
};
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::user::UpdateUser;
use crate::part::repo::user::UserRepo;
use crate::result::{BaseRest, ExpectedVariant};

/// Marks the authenticated user online in one team for ten minutes.
/// Updates user and membership activity before renewing the lease, without a transaction.
#[instrument(
    level = "info",
    skip(repo, cache, token),
    fields(actor_user_id = %token.user_id),
)]
pub async fn mark_self_online<C, R, K>(
    (repo, cache): (&R, &K),
    token: UserToken,
    team_id: String,
) -> BaseRest<()>
where
    C: Context,
    R: MemberRepo<C> + UserRepo<C> + Sync,
    K: OnlineUserCache + Sync,
{
    let member_info = FindMemberInfo {
        user_id: &token.user_id,
        team_id: &team_id,
    }
    .run_on(repo)
    .await?;

    let Some(member_info) = member_info else {
        //
        return Err(expected_error! {
            ExpectedVariant::Perm,
            message: trl("error-team-member-required");
            "team membership required",
        });
    };

    team_perm_complex::ensure_user_can_mark_self_online(&member_info)?;

    UpdateUser::TouchLastActive { id: &token.user_id }
        .run_on(repo)
        .await?;

    MarkUserOnline::new(&team_id, &token.user_id)
        .run_on(cache)
        .await?;

    accept!(())
}

/// Lists active user identifiers for one team in ascending order.
#[instrument(
    level = "info",
    skip(repo, cache, token),
    fields(actor_user_id = %token.user_id),
)]
pub async fn list_online_user_ids<C, R, K>(
    (repo, cache): (&R, &K),
    token: UserToken,
    team_id: String,
) -> BaseRest<Vec<String>>
where
    C: Context,
    R: MemberRepo<C> + Sync,
    K: OnlineUserCache + Sync,
{
    let member_info = FindMemberInfo {
        user_id: &token.user_id,
        team_id: &team_id,
    }
    .run_on(repo)
    .await?;

    let Some(member_info) = member_info else {
        //
        return Err(expected_error! {
            ExpectedVariant::Perm,
            message: trl("error-team-member-required");
            "team membership required",
        });
    };

    team_perm_complex::ensure_user_can_list_online_user_ids(&member_info)?;

    let online_user_ids =
        ListOnlineUserIds::new(&team_id).run_on(cache).await?;

    accept!(online_user_ids)
}
