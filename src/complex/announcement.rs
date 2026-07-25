//! Complex-domain opers for team announcements.

use poprako_orchestra::Proxy;

use crate::complex::util::{check_user_is_team_admin, check_user_is_team_member};
use crate::part::repo::oper::member::FindMemberInfo;
use crate::result::{BaseError, BaseResult};
use crate::util::next_snowflake_id;

/// Domain opers for announcements.
pub struct AnnouncementComplex;

impl AnnouncementComplex {
    /// Generate a unique announcement identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }
}

/// Permission-gate opers for announcements.
pub struct AnnouncementPermComplex;

impl AnnouncementPermComplex {
    /// Verify the caller may list announcements under the team.
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

    /// Verify the caller may create an announcement under the team.
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
}
