//! In-memory current review replacement and lifecycle cleanup.

use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::read::proj::issue::IssueInfo;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::issue::{
    ClearChapterIssues, ListIssueInfos, ReplaceChapterIssues,
};
use crate::part_impl::repo::mock_impl::{Mock, MockContext};
use crate::result::{BaseError, BaseRest, accept};

// Remove all details of the locked Chapter's current review.
fn clear(context: &mut MockContext, chapter_id: &str) {
    //
    let page_ids = context
        .state
        .pages
        .iter()
        .filter(|page| page.chapter_id == chapter_id)
        .map(|page| page.id.as_str())
        .collect::<Vec<_>>();

    context
        .state
        .issues
        .retain(|issue| !page_ids.contains(&issue.page_id.as_str()));
}

impl Run<ListIssueInfos<'_>> for Mock {
    // Application error returned by this operation.
    type Error = BaseError;

    // Executes this repository operation.
    #[instrument(level = "info", skip_all)]
    async fn run(&self, oper: &ListIssueInfos<'_>) -> BaseRest<Vec<IssueInfo>> {
        //
        let mut issues = self
            .state
            .lock()
            .unwrap()
            .issues
            .iter()
            .filter(|issue| issue.page_id == oper.page_id)
            .cloned()
            .collect::<Vec<_>>();

        issues.sort_by_key(|issue| issue.index);

        accept(issues)
    }
}

impl Step<ReplaceChapterIssues<'_>, MockContext> for Mock {
    // Minimum transaction isolation required by this operation.
    type Level = ReptRead;

    // Application error returned by this operation.
    type Error = BaseError;

    // Executes this repository operation.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ReplaceChapterIssues<'_>,
    ) -> BaseRest<()> {
        //
        clear(context, oper.repl.chapter_id);

        context
            .state
            .issues
            .extend(oper.repl.entries.iter().map(|entry| IssueInfo {
                id: entry.id.clone(),
                page_id: entry.page_id.clone(),
                index: entry.index,
                variant: entry.variant.clone(),
                layer_path: entry.layer_path.clone(),
                rect: entry.rect,
                note: entry.note.clone(),
            }));

        accept(())
    }
}

impl Step<ClearChapterIssues<'_>, MockContext> for Mock {
    // Minimum transaction isolation required by this operation.
    type Level = ReptRead;

    // Application error returned by this operation.
    type Error = BaseError;

    // Executes this repository operation.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ClearChapterIssues<'_>,
    ) -> BaseRest<()> {
        //
        clear(context, oper.chapter_id);

        accept(())
    }
}
