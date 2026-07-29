//! Complex domain logic for [Member] aggregates — ID generation and permission gates.

use poprako_orchestra::Proxy;

use crate::complex::util::{
    check_user_is_team_admin, check_user_is_team_member,
};
use crate::part::repo::oper::member::FindMemberInfo;
use crate::result::{BaseError, BaseResult};
use crate::util::next_snowflake_id;

/// Domain opers for [Member] aggregates: unique identifier generation.
pub struct MemberComplex;

impl MemberComplex {
    /// Generates a unique member identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }
}

/// Permission-gate opers for team membership — team-scoped.
pub struct MemberPermComplex;

impl MemberPermComplex {
    /// Verify the caller is a team admin of the given team.
    pub async fn ensure_user_can_update_info<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> BaseResult<()>
    where
        P: for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    /// Verify the caller is a team admin of the given team.
    pub async fn ensure_user_can_delete<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> BaseResult<()>
    where
        P: for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    /// Verify the caller is a team admin of the given team.
    pub async fn ensure_user_can_create<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> BaseResult<()>
    where
        P: for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    /// Verify the caller is a teammember.
    pub async fn ensure_user_can_list_infos<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> BaseResult<()>
    where
        P: for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        check_user_is_team_member(proxy, user_id, team_id).await
    }
}
