//! Shared helpers for the complex layer.

use poprako_util::i18n::trl;

use crate::model::role::RoleField;
use crate::part::repo::proxy::ProxyExecute;
use crate::part::repo::step::member::{FindByUserTeamId, MemberStep};
use crate::result::{ExpectedVariant, RootError, RootResult, accept};

/// Verify the user is a member of the given team; returns `Perm` error if not.
pub(super) async fn check_user_is_team_member<P>(
    proxy: &mut P,
    user_id: &str,
    team_id: &str,
) -> RootResult<()>
where
    P: for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
{
    let member_info = proxy
        .execute(&MemberStep::find_by_user_team_id(user_id, team_id))
        .await?;

    if member_info.is_none() {
        return Err(RootError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-team-member-required"),
        });
    }

    accept(())
}

/// Verify the user is a team admin; returns `Perm` error if not.
pub(super) async fn check_user_is_team_admin<P>(
    proxy: &mut P,
    user_id: &str,
    team_id: &str,
) -> RootResult<()>
where
    P: for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
{
    let member_info = proxy
        .execute(&MemberStep::find_by_user_team_id(user_id, team_id))
        .await?;

    let Some(member_info) = member_info else {
        return Err(RootError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-team-admin-required"),
        });
    };

    if !member_info.role_mask.has_any_role(&[RoleField::ADMIN]) {
        return Err(RootError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-team-admin-required"),
        });
    }

    accept(())
}
