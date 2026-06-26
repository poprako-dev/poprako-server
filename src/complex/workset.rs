//! Complex-domain operations for workset entities: identity generation and
//! recursive deletion with owned comic cleanup.

use uuid::Uuid;

use crate::complex::comic::ComicComplex;
use crate::complex::util::{check_user_is_team_admin, check_user_is_team_member};
use crate::part::prom::PromTransactional;
use crate::part::repo::comic::ComicRepoTransactional;
use crate::part::repo::proxy::ProxyExecute;
use crate::part::repo::step::comic::ComicStep;
use crate::part::repo::step::member::FindByUserTeamId;
use crate::part::repo::step::workset::{GetInfoById as WorksetGetInfoById, WorksetStep};
use crate::part::repo::workset::WorksetRepoTransactional;
use crate::result::{RootError, RootResult};

/// Domain operations for workset entities.
pub struct WorksetComplex;

impl WorksetComplex {
    /// Generate a unique, time-ordered workset identifier (e.g. `workset-<uuid-v7>`).
    pub fn gen_id() -> String {
        format!("workset-{}", Uuid::now_v7())
    }

    /// Recursively delete a workset and all owned resources: cascades into comic
    /// deletion for every comic in the workset, then deletes the workset record.
    pub async fn delete_cascade<C, R, P>(
        repo: &R,
        prom: &P,
        context: &mut C,
        id: &str,
    ) -> RootResult<()>
    where
        C: Send,
        R: WorksetRepoTransactional<C> + ComicRepoTransactional<C> + Send + Sync,
        P: PromTransactional<C> + Send + Sync,
    {
        let _ = repo
            .advance(context, &WorksetStep::get_info_excluded(id))
            .await?;

        let comic_infos = repo
            .advance(context, &ComicStep::list_by_workset_id_excluded(id))
            .await?;

        for comic_info in comic_infos {
            ComicComplex::delete_cascade(repo, prom, context, &comic_info.id).await?;
        }

        repo.advance(context, &WorksetStep::delete(id)).await?;

        Ok(())
    }
}

/// Permission-gate operations for workset entities — workset-scoped.
pub struct WorksetPermComplex;

impl WorksetPermComplex {
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

    pub async fn can_user_get_info<P>(
        proxy: &mut P,
        user_id: &str,
        workset_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        let team_id = Self::resolve_team_id(proxy, workset_id).await?;

        check_user_is_team_member(proxy, user_id, &team_id).await
    }

    pub async fn can_user_update_info<P>(
        proxy: &mut P,
        user_id: &str,
        workset_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        let team_id = Self::resolve_team_id(proxy, workset_id).await?;

        check_user_is_team_admin(proxy, user_id, &team_id).await
    }

    pub async fn can_user_delete<P>(
        proxy: &mut P,
        user_id: &str,
        workset_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        let team_id = Self::resolve_team_id(proxy, workset_id).await?;

        check_user_is_team_admin(proxy, user_id, &team_id).await
    }

    async fn resolve_team_id<P>(proxy: &mut P, workset_id: &str) -> RootResult<String>
    where
        P: for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>,
    {
        let workset_info = proxy
            .execute(&WorksetStep::get_info_by_id(workset_id))
            .await?;

        Ok(workset_info.team_id)
    }
}
