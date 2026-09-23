//! Test adapter that forces a real database failure after history was written.

use std::sync::atomic::{AtomicBool, Ordering};

use poprako_orchestra::{Oper, Run, Step};

use crate::part::nucl::ReptRead;
use crate::part::repo::oper::chapter_workflow_record::CreateChapterWorkflowRecords;
use crate::part::repo::oper::{
    assignment, chapter, comic, member, team, workset,
};
use crate::part_impl::repo::HybRepo;
use crate::result::{BaseError, BaseRest};
use crate::shared::RdbContext;

/// Delegates real writes, then duplicates history to abort the transaction.
pub struct DuplicateHistoryRepo {
    repo: HybRepo,
    history_written: AtomicBool,
}

impl DuplicateHistoryRepo {
    /// Wraps the real repository without changing its transaction coordinator.
    pub fn new(repo: HybRepo) -> Self {
        //
        Self {
            repo,
            history_written: AtomicBool::new(false),
        }
    }

    /// Reports whether the required late history insert actually succeeded.
    pub fn history_written(&self) -> bool {
        self.history_written.load(Ordering::SeqCst)
    }
}

impl<O> Run<O> for DuplicateHistoryRepo
where
    O: Oper + Sync,
    HybRepo: Run<O, Error = BaseError>,
{
    // Retains the real repository error contract.
    type Error = BaseError;

    // Executes independent reads through the real repository.
    async fn run(&self, oper: &O) -> BaseRest<O::Output> {
        self.repo.run(oper).await
    }
}

impl Step<CreateChapterWorkflowRecords<'_>, RdbContext>
    for DuplicateHistoryRepo
{
    // Retains the real history insert isolation requirement.
    type Level = ReptRead;

    // Retains the real repository error contract.
    type Error = BaseError;

    // Inserts real history, then violates its primary key in the same transaction.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateChapterWorkflowRecords<'_>,
    ) -> BaseRest<()> {
        //
        assert!(!oper.entries.is_empty());

        self.repo.step(context, oper).await?;

        self.history_written.store(true, Ordering::SeqCst);

        self.repo.step(context, oper).await
    }
}

// Forwards existing repository capabilities through the original context.
macro_rules! forward_steps {
    ($($oper:ty),* $(,)?) => {
        $(
            impl Step<$oper, RdbContext> for DuplicateHistoryRepo {
                // Preserves the required isolation level of the real adapter.
                type Level = ReptRead;

                // Retains the real repository error contract.
                type Error = BaseError;

                // Executes the operation using the caller-owned database transaction.
                async fn step(
                    &self,
                    context: &mut RdbContext,
                    oper: &$oper,
                ) -> BaseRest<<$oper as Oper>::Output> {
                    self.repo.step(context, oper).await
                }
            }
        )*
    };
}

forward_steps!(
    chapter::GetChapterInfo<'_, '_>,
    chapter::GetChapterInfoExcluded<'_, '_>,
    chapter::GetChapterUnitEditScopeExcluded<'_>,
    chapter::ListChapterInfosExcluded<'_>,
    chapter::LockChapters<'_>,
    chapter::FindPinnedChapterInfo<'_, '_>,
    chapter::CreateChapter<'_>,
    chapter::UpdateChapter<'_>,
    chapter::UpdateChapterStage<'_>,
    chapter::StartChapterStage<'_>,
    chapter::CompleteChapterRawProvide<'_>,
    chapter::SetChapterPageCountMetrics<'_>,
    chapter::AdjustChapterUnitCountDelta<'_>,
    chapter::UnpinOtherChapters<'_>,
    assignment::FindAssignmentInfo<'_, '_>,
    assignment::ListAssignmentInfos<'_, '_>,
    assignment::ListAssignmentInfosExcluded<'_>,
    assignment::CreateAssignment<'_>,
    assignment::UpdateAssignmentRoles<'_>,
    assignment::DeleteAssignments<'_>,
    comic::GetComicInfo<'_, '_>,
    comic::ListComicInfos<'_>,
    comic::GetComicInfoExcluded<'_, '_>,
    comic::CreateComic<'_>,
    comic::AllocComicChapterIndex<'_>,
    comic::UpdateComicChapterCount<'_>,
    comic::TouchComicLastActive<'_>,
    workset::GetWorksetInfo<'_>,
    workset::ListWorksetInfos<'_>,
    workset::CreateWorkset<'_>,
    workset::AllocWorksetComicIndex<'_>,
    workset::UpdateWorksetComicCount<'_>,
    member::CreateMember<'_>,
    member::UpdateMember<'_>,
    member::ListMemberInfos<'_>,
    member::FindMemberInfo<'_>,
    member::GetMemberInfo<'_, '_>,
    member::LockTeamMemberInfos<'_>,
    member::DeleteMember<'_>,
    member::DeleteUserMemberships<'_>,
    team::CreateTeam<'_>,
    team::UpdateTeam<'_>,
    team::GetTeamInfoExcluded<'_>,
    team::LockTeam<'_>,
    team::ResolveTeamId<'_>,
    team::AllocTeamWorksetIndex<'_>,
);
