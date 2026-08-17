//! Complex-domain opers for workset entities: identity generation and
//! perm gates.

use poprako_orchestra::{OperProxy as _, Proxy};

use crate::complex::comic::ComicComplex;
use crate::complex::util::{
    check_user_is_team_admin, check_user_is_team_member,
};
use crate::model::read::spec::comic::ComicListSpec;
use crate::part::prom::oper::{Defer, DeferBatch};
use crate::part::prom::payload::TaskPayload;
use crate::part::repo::oper::assignment::DeleteAssignments;
use crate::part::repo::oper::assignment_invitation::DeleteAssignmentInvitations;
use crate::part::repo::oper::chapter::{
    DeleteChapter, GetChapterInfoExcluded, ListChapterInfosExcluded,
    UnpinOtherChapters, UpdateChapter,
};
use crate::part::repo::oper::chapter_workflow_record::DeleteChapterWorkflowRecords;
use crate::part::repo::oper::comic::{
    DeleteComic, GetComicInfoExcluded, ListComicInfosExcluded,
    TouchComicLastActive, UpdateComicChapterCount,
};
use crate::part::repo::oper::comic_archive::DeleteComicArchives;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::page::{DeletePages, ListPageInfos};
use crate::part::repo::oper::term::DeleteTerms;
use crate::part::repo::oper::termbase::{
    DeleteTermbase, GetTermbaseInfoExcluded, ListTermbaseInfosExcluded,
};
use crate::part::repo::oper::workset::{
    DeleteWorkset, GetWorksetInfo, GetWorksetInfoExcluded,
    UpdateWorksetComicCount,
};
use crate::result::{BaseError, BaseRest, accept};
use crate::util::next_snowflake_id;

/// Domain opers for workset entities.
pub struct WorksetComplex;

impl WorksetComplex {
    /// Generate a unique, time-ordered workset identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }

    /// Deletes a workset subtree inside an existing transaction context.
    pub async fn delete_cascade<P>(proxy: &mut P, id: &str) -> BaseRest<()>
    where
        P: for<'a> Proxy<GetWorksetInfoExcluded<'a>, Error = BaseError>
            + for<'a> Proxy<ListComicInfosExcluded<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteWorkset<'a>, Error = BaseError>
            + for<'a, 'b> Proxy<GetComicInfoExcluded<'a, 'b>, Error = BaseError>
            + for<'a> Proxy<ListChapterInfosExcluded<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteComic<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteComicArchives<'a>, Error = BaseError>
            + for<'a> Proxy<UpdateWorksetComicCount<'a>, Error = BaseError>
            + for<'a, 'b> Proxy<GetChapterInfoExcluded<'a, 'b>, Error = BaseError>
            + for<'a> Proxy<ListPageInfos<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteAssignmentInvitations<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteAssignments<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteChapterWorkflowRecords<'a>, Error = BaseError>
            + for<'a> Proxy<DeletePages<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteChapter<'a>, Error = BaseError>
            + for<'a> Proxy<UpdateChapter<'a>, Error = BaseError>
            + for<'a> Proxy<UnpinOtherChapters<'a>, Error = BaseError>
            + for<'a> Proxy<UpdateComicChapterCount<'a>, Error = BaseError>
            + for<'a> Proxy<TouchComicLastActive<'a>, Error = BaseError>
            + for<'a> Proxy<ListTermbaseInfosExcluded<'a>, Error = BaseError>
            + for<'a> Proxy<GetTermbaseInfoExcluded<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteTerms<'a>, Error = BaseError>
            + for<'a> Proxy<DeleteTermbase<'a>, Error = BaseError>
            + for<'a> Proxy<Defer<'a, String, TaskPayload, ()>, Error = BaseError>
            + for<'t, 'a> Proxy<
                DeferBatch<'t, 'a, String, TaskPayload, ()>,
                Error = BaseError,
            >,
    {
        // SAFETY: Lock the root workset row (FOR UPDATE) to serialize with
        // concurrent comic creations (IncrComicNextIndex also locks this row),
        // preventing resource leaks from comics inserted between the last
        // paginated page and the workset delete.

        let workset_info =
            GetWorksetInfoExcluded { id }.proxy_on(proxy).await?;

        // Page size for paginated comic deletion cascades.
        const PAGE_SIZE: u32 = 50;

        loop {
            //
            let list_spec = ComicListSpec {
                workset_id: workset_info.id.clone(),
                fuzzy_title: None,
                stages: None,
                status: None,
                incl_opt: Vec::new(),
                offset: 0,
                limit: PAGE_SIZE,
            };

            let comic_infos = ListComicInfosExcluded { spec: &list_spec }
                .proxy_on(proxy)
                .await?;

            if comic_infos.is_empty() {
                break;
            }

            for comic_info in comic_infos {
                ComicComplex::delete_cascade(proxy, &comic_info.id).await?;
            }
        }

        DeleteWorkset {
            id: &workset_info.id,
        }
        .proxy_on(proxy)
        .await?;

        accept(())
    }
}

/// perm-gate opers for workset entities — workset-scoped.
pub struct WorksetPermComplex;

impl WorksetPermComplex {
    /// Verify the caller is a team admin.
    pub async fn ensure_user_can_create<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> BaseRest<()>
    where
        P: for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        check_user_is_team_admin(proxy, user_id, team_id).await
    }

    /// Verify the caller is a team member.
    pub async fn ensure_user_can_list_infos<P>(
        proxy: &mut P,
        user_id: &str,
        team_id: &str,
    ) -> BaseRest<()>
    where
        P: for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        check_user_is_team_member(proxy, user_id, team_id).await
    }

    /// Verify the caller is a team member of the workset's team.
    pub async fn ensure_user_can_get_info<P>(
        proxy: &mut P,
        user_id: &str,
        workset_id: &str,
    ) -> BaseRest<()>
    where
        P: for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        let workset_info =
            GetWorksetInfo { id: workset_id }.proxy_on(proxy).await?;

        check_user_is_team_member(proxy, user_id, &workset_info.team_id).await
    }

    /// Verify the caller is a team admin of the workset's team.
    pub async fn ensure_user_can_update_info<P>(
        proxy: &mut P,
        user_id: &str,
        workset_id: &str,
    ) -> BaseRest<()>
    where
        P: for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        let workset_info =
            GetWorksetInfo { id: workset_id }.proxy_on(proxy).await?;

        check_user_is_team_admin(proxy, user_id, &workset_info.team_id).await
    }

    /// Verify the caller is a team admin of the workset's team.
    pub async fn ensure_user_can_delete<P>(
        proxy: &mut P,
        user_id: &str,
        workset_id: &str,
    ) -> BaseRest<()>
    where
        P: for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        let workset_info =
            GetWorksetInfo { id: workset_id }.proxy_on(proxy).await?;

        check_user_is_team_admin(proxy, user_id, &workset_info.team_id).await
    }
}
