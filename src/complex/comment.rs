//! Complex-domain opers for team comments.

use poprako_orchestra::Proxy;

use crate::complex::util::check_user_is_team_member;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::result::{RegularError, RegularResult};
use crate::util::next_snowflake_id;

/// Domain opers for comments.
pub struct CommentComplex;

impl CommentComplex {
    /// Generate a unique comment identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }
}

/// Permission-gate opers for comments.
pub struct CommentPermComplex;

impl CommentPermComplex {
    /// Verify the caller may list comments under the team.
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

    /// Verify the caller may create a comment under the team.
    pub async fn can_user_create<P>(
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
