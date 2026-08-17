//! Workset use cases — create, read, update, list, and deletion.

/// Workset use-case test helpers.
#[cfg(test)]
pub mod tests;

use poprako_orchestra::{
    AtLeast, Context, Nucl, OperRun as _, OperStep as _, run_proxy, step_proxy,
};
use tracing::instrument;

use crate::complex::workset::{WorksetComplex, WorksetPermComplex};
use crate::data::instr::workset::{
    CreateWorksetInstr, ListWorksetInfosInstr, UpdateWorksetInfoInstr,
};
use crate::data::val::workset::CreateWorksetVal;
use crate::data::view::workset::WorksetInfoView;
use crate::model::shared::user::UserToken;
use crate::model::write::workset::{WorksetEntry, WorksetRepl};
use crate::part::nucl::{RepeatableRead, Serializable};
use crate::part::prom::Prom;
use crate::part::prom::oper::{Defer, DeferBatch};
use crate::part::prom::payload::TaskPayload;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::assignment_invitation::AssignmentInvitationRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::comic_archive::ComicArchiveRepo;
use crate::part::repo::member::MemberRepo;
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
use crate::part::repo::oper::team::AllocTeamWorksetIndex;
use crate::part::repo::oper::term::DeleteTerms;
use crate::part::repo::oper::termbase::{
    DeleteTermbase, GetTermbaseInfoExcluded, ListTermbaseInfosExcluded,
};
use crate::part::repo::oper::workset::{
    CreateWorkset, DeleteWorkset, GetWorksetInfo, GetWorksetInfoExcluded,
    ListWorksetInfos, UpdateWorkset, UpdateWorksetComicCount,
};
use crate::part::repo::page::PageRepo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::term::TermRepo;
use crate::part::repo::termbase::TermbaseRepo;
use crate::part::repo::unit::UnitRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{BaseError, BaseRest, accept};

/// Creates a new workset inside a team.
#[instrument(level = "info", skip(nucl, repo))]
pub async fn create<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: CreateWorksetInstr,
) -> BaseRest<CreateWorksetVal>
where
    C: Context,
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    C::Level: AtLeast<RepeatableRead>,
    R: TeamRepo<C> + WorksetRepo<C> + MemberRepo<C> + Send + Sync,
{
    WorksetPermComplex::ensure_user_can_create(
        &mut run_proxy! {
            repo => for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &instr.team_id,
    )
    .await?;

    let workset_id = nucl
        .coord(async move |context| {
            //
            let index = AllocTeamWorksetIndex { id: &instr.team_id }
                .step_on(repo, context)
                .await?;

            let workset_entry = WorksetEntry {
                id: WorksetComplex::gen_id(),
                team_id: instr.team_id,
                index,
                name: instr.name,
                description: instr.description,
            };

            let workset_info = CreateWorkset {
                entry: &workset_entry,
            }
            .step_on(repo, context)
            .await?;

            accept(workset_info.id)
        })
        .await?;

    accept(CreateWorksetVal { id: workset_id })
}

/// Fetches a workset by ID.
#[instrument(level = "info", skip(repo))]
pub async fn get_info<C, R>(
    (repo,): (&R,),
    token: UserToken,
    id: String,
) -> BaseRest<WorksetInfoView>
where
    C: Context,
    R: WorksetRepo<C> + MemberRepo<C> + Sync,
{
    WorksetPermComplex::ensure_user_can_get_info(
        &mut run_proxy! {
            repo =>
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &id,
    )
    .await?;

    let workset_info = GetWorksetInfo { id: &id }.run_on(repo).await?;

    accept(workset_info.into())
}

/// Lists worksets for a team.
#[instrument(level = "info", skip(repo))]
pub async fn list_infos<C, R>(
    (repo,): (&R,),
    token: UserToken,
    instr: ListWorksetInfosInstr,
) -> BaseRest<Vec<WorksetInfoView>>
where
    C: Context,
    R: WorksetRepo<C> + MemberRepo<C> + Sync,
{
    WorksetPermComplex::ensure_user_can_list_infos(
        &mut run_proxy! {
            repo => for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &instr.team_id,
    )
    .await?;

    let workset_infos = ListWorksetInfos {
        team_id: &instr.team_id,
        offset: instr.offset,
        limit: instr.limit,
    }
    .run_on(repo)
    .await?;

    accept(workset_infos.into_iter().map(Into::into).collect())
}

/// Updates a workset's name and description.
#[instrument(level = "info", skip(repo))]
pub async fn update_info<C, R>(
    (repo,): (&R,),
    token: UserToken,
    instr: UpdateWorksetInfoInstr,
) -> BaseRest<()>
where
    C: Context,
    R: WorksetRepo<C> + MemberRepo<C> + Sync,
{
    WorksetPermComplex::ensure_user_can_update_info(
        &mut run_proxy! {
            repo =>
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &instr.id,
    )
    .await?;

    let workset_info_update = WorksetRepl {
        id: instr.id,
        name: instr.name,
        description: instr.description,
    };

    UpdateWorkset {
        update: &workset_info_update,
    }
    .run_on(repo)
    .await?;

    accept(())
}

/// Deletes a workset and its child data.
#[instrument(level = "info", skip(nucl, repo, prom))]
pub async fn delete<N, C, R, P>(
    (nucl, repo, prom): (&N, &R, &P),
    token: UserToken,
    id: String,
) -> BaseRest<()>
where
    C: Context,
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    C::Level: AtLeast<Serializable>,
    R: WorksetRepo<C>
        + ComicRepo<C>
        + ComicArchiveRepo<C>
        + MemberRepo<C>
        + ChapterRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + PageRepo<C>
        + AssignmentInvitationRepo<C>
        + AssignmentRepo<C>
        + UnitRepo<C>
        + TermbaseRepo<C>
        + TermRepo<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
{
    WorksetPermComplex::ensure_user_can_delete(
        &mut run_proxy! {
            repo =>
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &id,
    )
    .await?;

    nucl.coord(async move |context| {
        //
        let guarded_repo = &crate::part::nucl::GuardedStep::new(repo);

        let guarded_prom = &crate::part::nucl::GuardedStep::new(prom);

        WorksetComplex::delete_cascade(
            &mut step_proxy! {
                context;
                guarded_repo =>
                    for<'a> GetWorksetInfoExcluded<'a>,
                    for<'a> ListComicInfosExcluded<'a>,
                    for<'a> DeleteWorkset<'a>,
                    for<'a, 'b> GetComicInfoExcluded<'a, 'b>,
                    for<'a> ListChapterInfosExcluded<'a>,
                    for<'a> DeleteComic<'a>,
                    for<'a> DeleteComicArchives<'a>,
                    for<'a> UpdateWorksetComicCount<'a>,
                    for<'a, 'b> GetChapterInfoExcluded<'a, 'b>,
                    for<'a> ListPageInfos<'a>,
                    for<'a> DeleteAssignmentInvitations<'a>,
                    for<'a> DeleteAssignments<'a>,
                    for<'a> DeleteChapterWorkflowRecords<'a>,
                    for<'a> DeletePages<'a>,
                    for<'a> DeleteChapter<'a>,
                    for<'a> UpdateChapter<'a>,
                    for<'a> UnpinOtherChapters<'a>,
                    for<'a> UpdateComicChapterCount<'a>,
                    for<'a> TouchComicLastActive<'a>,
                    for<'a> ListTermbaseInfosExcluded<'a>,
                    for<'a> GetTermbaseInfoExcluded<'a>,
                    for<'a> DeleteTerms<'a>,
                    for<'a> DeleteTermbase<'a>;
                guarded_prom =>
                    for<'a> Defer<'a, String, TaskPayload, ()>,
                    for<'t, 'a> DeferBatch<'t, 'a, String, TaskPayload, ()>;
            },
            &id,
        )
        .await?;

        accept(())
    })
    .await?;

    accept(())
}
