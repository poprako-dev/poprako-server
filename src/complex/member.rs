//! Complex domain logic for [Member] aggregates — ID generation and permission gates.

use poprako_orchestra::Proxy;

use crate::complex::util::{
    check_user_is_team_admin, check_user_is_team_member,
};
use crate::part::repo::oper::member::FindMemberInfo;
use crate::result::{RegularError, RegularResult};
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
    pub async fn can_user_update_info<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> Proxy<FindMemberInfo<'a>, Error = RegularError>,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    /// Verify the caller is a team admin of the given team.
    pub async fn can_user_delete<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> Proxy<FindMemberInfo<'a>, Error = RegularError>,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    /// Verify the caller is a team admin of the given team.
    pub async fn can_user_create<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> Proxy<FindMemberInfo<'a>, Error = RegularError>,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    /// Verify the caller is a teammember.
    pub async fn can_user_list_infos<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> Proxy<FindMemberInfo<'a>, Error = RegularError>,
    {
        check_user_is_team_member(proxy, user_id, team_id).await
    }
}
