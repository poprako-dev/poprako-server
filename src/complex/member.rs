//! Complex domain logic for [Member] aggregates — ID generation and permission gates.

use uuid::Uuid;

use crate::complex::util::{check_user_is_team_admin, check_user_is_team_member};
use crate::part::repo::proxy::ProxyExecute;
use crate::part::repo::step::member::FindByUserTeamId;
use crate::result::{RootError, RootResult};

/// Domain operations for [Member] aggregates: unique identifier generation.
pub struct MemberComplex;

impl MemberComplex {
    /// Generates a unique member identifier with a `member-` prefix using UUID v7.
    pub fn gen_id() -> String {
        format!("member-{}", Uuid::now_v7())
    }
}

/// Permission-gate operations for team membership — team-scoped.
pub struct MemberPermComplex;

impl MemberPermComplex {
    // ── public ───────────────────────────────────────────────────────

    pub async fn can_user_update_info<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    pub async fn can_user_reserve_avatar<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    pub async fn can_user_mark_avatar_uploaded<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    pub async fn can_user_delete<P>(proxy: &mut P, user_id: &str, team_id: &str) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    pub async fn can_user_create<P>(proxy: &mut P, user_id: &str, team_id: &str) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    pub async fn can_user_list_infos<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        check_user_is_team_member(proxy, user_id, team_id).await
    }
}
