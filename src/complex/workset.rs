//! Complex-domain operations for workset entities: identity generation and
//! permission gates.

use crate::complex::comic::ComicComplex;
use crate::complex::util::{check_user_is_team_admin, check_user_is_team_member};
use crate::part::prom::PromTransactional;
use crate::part::repo::chapter::ChapterRepoTransactional;
use crate::part::repo::comic::ComicRepoTransactional;
use crate::part::repo::page::PageRepoTransactional;
use crate::part::repo::step::comic::ComicStep;
use crate::part::repo::step::member::FindInfoByUserTeamId;
use crate::part::repo::step::workset::{GetInfoById as WorksetGetInfoById, WorksetStep};
use crate::part::repo::workset::WorksetRepoTransactional;
use crate::part::shared::proxy::ProxyExecute;
use crate::result::{RootError, RootResult, accept};
use crate::util::next_snowflake_id;

/// Domain operations for workset entities.
pub struct WorksetComplex;

impl WorksetComplex {
    /// Generate a unique, time-ordered workset identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }

    /// Deletes a workset subtree inside an existing transaction context.
    pub async fn delete_cascade<C, R, P>(
        repo: &R,
        prom: &P,
        context: &mut C,
        id: &str,
    ) -> RootResult<()>
    where
        C: Send,
        R: WorksetRepoTransactional<C>
            + ComicRepoTransactional<C>
            + ChapterRepoTransactional<C>
            + PageRepoTransactional<C>
            + Send
            + Sync,
        P: PromTransactional<C> + Send + Sync,
    {
        let workset_info = repo
            .advance(context, &WorksetStep::get_info_excluded(id))
            .await?;

        let comic_infos = repo
            .advance(
                context,
                &ComicStep::list_infos_by_workset_id_excluded(&workset_info.id),
            )
            .await?;

        for comic_info in comic_infos {
            ComicComplex::delete_cascade(repo, prom, context, &comic_info.id).await?;
        }

        repo.advance(context, &WorksetStep::delete(&workset_info.id))
            .await?;

        accept(())
    }
}

/// Permission-gate operations for workset entities — workset-scoped.
pub struct WorksetPermComplex;

impl WorksetPermComplex {
    /// Verify the caller is a team admin.
    pub async fn can_user_create<P>(proxy: &mut P, user_id: &str, team_id: &str) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<FindInfoByUserTeamId<'a>, Error = RootError>,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    /// Verify the caller is a team member.
    pub async fn can_user_list_infos<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<FindInfoByUserTeamId<'a>, Error = RootError>,
    {
        check_user_is_team_member(proxy, user_id, team_id).await
    }

    /// Verify the caller is a team member of the workset's team.
    pub async fn can_user_get_info<P>(
        proxy: &mut P,
        user_id: &str,
        workset_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<FindInfoByUserTeamId<'a>, Error = RootError>,
    {
        let team_id = Self::resolve_team_id(proxy, workset_id).await?;

        check_user_is_team_member(proxy, user_id, &team_id).await
    }

    /// Verify the caller is a team admin of the workset's team.
    pub async fn can_user_update_info<P>(
        proxy: &mut P,
        user_id: &str,
        workset_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<FindInfoByUserTeamId<'a>, Error = RootError>,
    {
        let team_id = Self::resolve_team_id(proxy, workset_id).await?;

        check_user_is_team_admin(proxy, user_id, &team_id).await
    }

    /// Verify the caller is a team admin of the workset's team.
    pub async fn can_user_delete<P>(
        proxy: &mut P,
        user_id: &str,
        workset_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<FindInfoByUserTeamId<'a>, Error = RootError>,
    {
        let team_id = Self::resolve_team_id(proxy, workset_id).await?;

        check_user_is_team_admin(proxy, user_id, &team_id).await
    }

    /// Resolve the owning team ID from a workset ID.
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
