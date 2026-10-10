//! In-memory current review replacement and lifecycle cleanup.

use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::read::proj::issue::IssueInfo;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::issue::{ListIssueInfos, ReplaceChapterIssues};
use crate::part_impl::repo::mock_impl::{Mock, MockContext};
use crate::result::{BaseError, BaseRest, accept};

// Remove all details of the locked Chapter's current review.
fn clear(context: &mut MockContext, chapter_id: &str) {
    //
    let ids = context
        .state
        .page_artworks
        .iter()
        .filter(|info| info.chapter_id == chapter_id)
        .map(|info| info.id.clone())
        .collect::<Vec<_>>();

    context
        .state
        .issues
        .retain(|issue| !ids.contains(&issue.page_artwork_id));
}

impl Run<ListIssueInfos<'_>> for Mock {
    // Application error returned by this operation.
    type Error = BaseError;

    // Executes this repository operation.
    #[instrument(level = "info", skip_all)]
    async fn run(&self, oper: &ListIssueInfos<'_>) -> BaseRest<Vec<IssueInfo>> {
        //
        let state = self.state.lock().unwrap();

        let mut issues = state
            .issues
            .iter()
            .filter(|issue| {
                //
                state.page_artworks.iter().any(|info| {
                    //
                    if info.chapter_id != oper.chapter_id {
                        return false;
                    }

                    info.id == issue.page_artwork_id
                })
            })
            .cloned()
            .collect::<Vec<_>>();

        issues.sort_by_key(|issue| {
            //
            (
                state
                    .page_artworks
                    .iter()
                    .find(|info| info.id == issue.page_artwork_id)
                    .map(|info| info.index),
                issue.index,
            )
        });

        drop(state);

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
                page_artwork_id: entry.page_artwork_id.clone(),
                index: entry.index,
                variant: entry.variant.clone(),
                layer_path: entry.layer_path.clone(),
                rect: entry.rect,
                note: entry.note.clone(),
            }));

        accept(())
    }
}
