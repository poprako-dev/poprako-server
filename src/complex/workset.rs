//! Complex-domain opers for workset entities: identity generation and
//! permission gates.

use crate::complex::comic::ComicComplex;
use crate::complex::util::{
    check_user_is_team_admin, check_user_is_team_member,
};
use crate::model::comic_model;
use crate::part::prom::Prom;
use crate::part::repo::assignment::AssignmentRepoTransactional;
use crate::part::repo::assignment_invitation::AssignmentInvitationRepoTransactional;
use crate::part::repo::chapter::ChapterRepoTransactional;
use crate::part::repo::comic::ComicRepoTransactional;
use crate::part::repo::page::PageRepoTransactional;
use crate::part::repo::step::comic::ComicStep;
use crate::part::repo::step::member::FindInfoByUserIdAndTeamId;
use crate::part::repo::step::workset::{
    GetInfoById as WorksetGetInfoById, WorksetStep,
};
use crate::part::repo::unit::UnitRepoTransactional;
use crate::part::repo::workset::WorksetRepoTransactional;
use crate::part::shared::proxy::ProxyExecute;
use crate::result::{RegularError, RegularResult};
use crate::util::next_snowflake_id;

/// Domain opers for workset entities.
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
    ) -> RegularResult<()>
    where
        C: Send,
        R: WorksetRepoTransactional<C>
            + ComicRepoTransactional<C>
            + ChapterRepoTransactional<C>
            + PageRepoTransactional<C>
            + AssignmentInvitationRepoTransactional<C>
            + AssignmentRepoTransactional<C>
            + UnitRepoTransactional<C>
            + Send
            + Sync,
        P: Prom<C> + Send + Sync,
    {
        // SAFETY: Lock the root workset row (FOR UPDATE) to serialize with
        // concurrent comic creations (IncrComicNextIndex also locks this row),
        // preventing resource leaks from comics inserted between the last
        // paginated page and the workset delete.
        let workset_info = repo
            .advance(context, &WorksetStep::get_info_excluded(id))
            .await?;

        const PAGE_SIZE: u64 = 50;

        let mut offset: u64 = 0;

        loop {
            //
            let list_spec = comic_model::ListSpec {
                workset_id: workset_info.id.clone(),
                fuzzy_title: None,
                kind: comic_model::ListKind::All,
                incl_opt: Vec::new(),
                offset,
                limit: PAGE_SIZE,
            };

            let comic_infos = repo
                .advance(context, &ComicStep::list_infos_excluded(&list_spec))
                .await?;

            if comic_infos.is_empty() {
                break;
            }

            for comic_info in comic_infos {
                ComicComplex::delete_cascade(
                    repo,
                    prom,
                    context,
                    &comic_info.id,
                )
                .await?;
            }

            offset += PAGE_SIZE;
        }

        repo.advance(context, &WorksetStep::delete(&workset_info.id))
            .await?;

        Ok(())
    }
}

/// Permission-gate opers for workset entities — workset-scoped.
pub struct WorksetPermComplex;

impl WorksetPermComplex {
    /// Verify the caller is a team admin.
    pub async fn can_user_create<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> ProxyExecute<
                FindInfoByUserIdAndTeamId<'a>,
                Error = RegularError,
            >,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    /// Verify the caller is a team member.
    pub async fn can_user_list_infos<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> ProxyExecute<
                FindInfoByUserIdAndTeamId<'a>,
                Error = RegularError,
            >,
    {
        check_user_is_team_member(proxy, user_id, team_id).await
    }

    /// Verify the caller is a team member of the workset's team.
    pub async fn can_user_get_info<P>(
        proxy: &mut P,
        user_id: &str,
        workset_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RegularError>
            + for<'a> ProxyExecute<
                FindInfoByUserIdAndTeamId<'a>,
                Error = RegularError,
            >,
    {
        let team_id = Self::resolve_team_id(proxy, workset_id).await?;

        check_user_is_team_member(proxy, user_id, &team_id).await
    }

    /// Verify the caller is a team admin of the workset's team.
    pub async fn can_user_update_info<P>(
        proxy: &mut P,
        user_id: &str,
        workset_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RegularError>
            + for<'a> ProxyExecute<
                FindInfoByUserIdAndTeamId<'a>,
                Error = RegularError,
            >,
    {
        let team_id = Self::resolve_team_id(proxy, workset_id).await?;

        check_user_is_team_admin(proxy, user_id, &team_id).await
    }

    /// Verify the caller is a team admin of the workset's team.
    pub async fn can_user_delete<P>(
        proxy: &mut P,
        user_id: &str,
        workset_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RegularError>
            + for<'a> ProxyExecute<
                FindInfoByUserIdAndTeamId<'a>,
                Error = RegularError,
            >,
    {
        let team_id = Self::resolve_team_id(proxy, workset_id).await?;

        check_user_is_team_admin(proxy, user_id, &team_id).await
    }

    /// Resolve the owning team ID from a workset ID.
    async fn resolve_team_id<P>(
        proxy: &mut P,
        workset_id: &str,
    ) -> RegularResult<String>
    where
        P: for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RegularError>,
    {
        let workset_info = proxy
            .execute(&WorksetStep::get_info_by_id(workset_id))
            .await?;

        Ok(workset_info.team_id)
    }
}
