//! In-memory hierarchy mark-and-sweep operations.

// Claims tombstones and physically removes their direct dependants.
mod sweep;

use std::collections::HashSet;

use poprako_orchestra::Step;
use tracing::instrument;

use crate::model::read::proj::subtree_delete::{
    SubtreeDeleteScope, SubtreeDeleteSweepTarget,
};
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::subtree_delete::{
    ClaimSubtreeSweep, DeleteSubtree, ListSubtreePageIds,
    LockSubtreeDeleteScope, MarkSubtree, SubtreeRoot, SweepSubtree,
};
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, expected,
};
use crate::result::{BaseError, BaseRest, accept};

/// Collect worksets covered by a deletion scope.
pub fn workset_ids(
    state: &MockState,
    scope: &SubtreeDeleteScope,
) -> HashSet<String> {
    //
    state
        .worksets
        .iter()
        .filter(|workset_info| match scope {
            //
            SubtreeDeleteScope::Team { team_id } => {
                workset_info.team_id == *team_id
            }

            SubtreeDeleteScope::Workset { workset_id, .. } => {
                workset_info.id == *workset_id
            }

            SubtreeDeleteScope::Comic { .. }
            | SubtreeDeleteScope::Chapter { .. } => false,
        })
        .map(|workset_info| workset_info.id.clone())
        .collect()
}

/// Collect comics covered by a deletion scope.
pub fn comic_ids(
    state: &MockState,
    scope: &SubtreeDeleteScope,
) -> HashSet<String> {
    //
    let workset_ids = workset_ids(state, scope);

    state
        .comics
        .iter()
        .filter(|comic_info| match scope {
            //
            SubtreeDeleteScope::Team { .. }
            | SubtreeDeleteScope::Workset { .. } => {
                workset_ids.contains(&comic_info.workset_id)
            }

            SubtreeDeleteScope::Comic { comic_id, .. }
            | SubtreeDeleteScope::Chapter { comic_id, .. } => {
                comic_info.id == *comic_id
            }
        })
        .map(|comic_info| comic_info.id.clone())
        .collect()
}

/// Collect chapters covered by a deletion scope.
pub fn chapter_ids(
    state: &MockState,
    scope: &SubtreeDeleteScope,
) -> HashSet<String> {
    //
    let comic_ids = comic_ids(state, scope);

    state
        .chapters
        .iter()
        .filter(|chapter_info| match scope {
            //
            SubtreeDeleteScope::Chapter { chapter_id, .. } => {
                chapter_info.id == *chapter_id
            }

            SubtreeDeleteScope::Team { .. }
            | SubtreeDeleteScope::Workset { .. }
            | SubtreeDeleteScope::Comic { .. } => {
                comic_ids.contains(&chapter_info.comic_id)
            }
        })
        .map(|chapter_info| chapter_info.id.clone())
        .collect()
}

/// Collect pages owned by a chapter.
pub(super) fn page_ids(state: &MockState, chapter_id: &str) -> HashSet<String> {
    //
    state
        .pages
        .iter()
        .filter(|page_info| page_info.chapter_id == chapter_id)
        .map(|page_info| page_info.id.clone())
        .collect()
}

/// Lock and resolve an active deletion root.
pub fn lock_scope(
    state: &MockState,
    root: &SubtreeRoot<'_>,
) -> BaseRest<SubtreeDeleteScope> {
    //
    match root {
        //
        SubtreeRoot::Team { id } => {
            //
            let team_info = state
                .teams
                .iter()
                .find(|team_info| {
                    team_info.id == *id && !state.deleted_team_ids.contains(*id)
                })
                .ok_or_else(|| expected("error-team-not-found"))?;

            accept(SubtreeDeleteScope::Team {
                team_id: team_info.id.clone(),
            })
        }

        SubtreeRoot::Workset { id } => {
            //
            let workset_info = state
                .worksets
                .iter()
                .find(|workset_info| {
                    //
                    workset_info.id == *id
                        && !state.deleted_workset_ids.contains(*id)
                })
                .ok_or_else(|| expected("error-workset-not-found"))?;

            accept(SubtreeDeleteScope::Workset {
                workset_id: workset_info.id.clone(),
                team_id: workset_info.team_id.clone(),
            })
        }

        SubtreeRoot::Comic { id } => {
            //
            let comic_info = state
                .comics
                .iter()
                .find(|comic_info| {
                    //
                    comic_info.id == *id
                        && !state.deleted_comic_ids.contains(*id)
                })
                .ok_or_else(|| expected("error-comic-not-found"))?;

            let workset_info = state
                .worksets
                .iter()
                .find(|workset_info| workset_info.id == comic_info.workset_id)
                .ok_or_else(|| expected("error-workset-not-found"))?;

            accept(SubtreeDeleteScope::Comic {
                comic_id: comic_info.id.clone(),
                workset_id: workset_info.id.clone(),
                team_id: workset_info.team_id.clone(),
            })
        }

        SubtreeRoot::Chapter { id } => {
            //
            let chapter_info = state
                .chapters
                .iter()
                .find(|chapter_info| {
                    //
                    chapter_info.id == *id
                        && !state.deleted_chapter_ids.contains(*id)
                })
                .ok_or_else(|| expected("error-chapter-not-found"))?;

            let comic_info = state
                .comics
                .iter()
                .find(|comic_info| comic_info.id == chapter_info.comic_id)
                .ok_or_else(|| expected("error-comic-not-found"))?;

            let workset_info = state
                .worksets
                .iter()
                .find(|workset_info| workset_info.id == comic_info.workset_id)
                .ok_or_else(|| expected("error-workset-not-found"))?;

            accept(SubtreeDeleteScope::Chapter {
                chapter_id: chapter_info.id.clone(),
                comic_id: comic_info.id.clone(),
                workset_id: workset_info.id.clone(),
                team_id: workset_info.team_id.clone(),
                was_pinned: chapter_info.is_pinned,
            })
        }
    }
}

/// Mark a deletion scope and its descendants.
pub fn mark_scope(
    state: &mut MockState,
    scope: &SubtreeDeleteScope,
) -> BaseRest<()> {
    //
    let marked_workset_ids = workset_ids(state, scope);

    let marked_comic_ids = comic_ids(state, scope);

    let marked_chapter_ids = chapter_ids(state, scope);

    match scope {
        //
        SubtreeDeleteScope::Team { team_id } => {
            state.deleted_team_ids.insert(team_id.clone());
        }

        SubtreeDeleteScope::Workset { workset_id, .. } => {
            state.deleted_workset_ids.insert(workset_id.clone());
        }

        SubtreeDeleteScope::Comic { comic_id, .. } => {
            state.deleted_comic_ids.insert(comic_id.clone());
        }

        SubtreeDeleteScope::Chapter { .. } => {
            //
            return Err(BaseError::Unrecoverable {
                message: "direct chapter deletion must not create a tombstone"
                    .into(),
            });
        }
    }

    state.deleted_workset_ids.extend(marked_workset_ids);

    state.deleted_comic_ids.extend(marked_comic_ids);

    state.deleted_chapter_ids.extend(marked_chapter_ids);

    accept(())
}

impl Step<LockSubtreeDeleteScope<'_>, MockContext> for Mock {
    // Required Orchestra execution level.
    type Level = ReptRead;
    // Shared repository error type.
    type Error = BaseError;

    // Execute the repository operation.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &LockSubtreeDeleteScope<'_>,
    ) -> BaseRest<SubtreeDeleteScope> {
        lock_scope(&context.state, &oper.root)
    }
}

impl Step<MarkSubtree<'_>, MockContext> for Mock {
    // Required Orchestra execution level.
    type Level = ReptRead;
    // Shared repository error type.
    type Error = BaseError;

    // Execute the repository operation.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &MarkSubtree<'_>,
    ) -> BaseRest<()> {
        mark_scope(&mut context.state, oper.scope)
    }
}

impl Step<ClaimSubtreeSweep, MockContext> for Mock {
    // Required Orchestra execution level.
    type Level = ReptRead;
    // Shared repository error type.
    type Error = BaseError;

    // Execute the repository operation.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ClaimSubtreeSweep,
    ) -> BaseRest<Option<SubtreeDeleteSweepTarget>> {
        accept(sweep::claim(&context.state, oper.level))
    }
}

impl Step<ListSubtreePageIds<'_>, MockContext> for Mock {
    // Required Orchestra execution level.
    type Level = ReptRead;
    // Shared repository error type.
    type Error = BaseError;

    // Execute the repository operation.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListSubtreePageIds<'_>,
    ) -> BaseRest<Vec<String>> {
        //
        let mut ids = page_ids(&context.state, oper.chapter_id)
            .into_iter()
            .collect::<Vec<_>>();

        ids.sort_unstable();

        accept(ids)
    }
}

impl Step<DeleteSubtree<'_>, MockContext> for Mock {
    // Required Orchestra execution level.
    type Level = ReptRead;
    // Shared repository error type.
    type Error = BaseError;

    // Execute the repository operation.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeleteSubtree<'_>,
    ) -> BaseRest<()> {
        //
        let SubtreeDeleteScope::Chapter { chapter_id, .. } = oper.scope else {
            //
            return Err(BaseError::Unrecoverable {
                message: "only a direct chapter may bypass subtree tombstones"
                    .into(),
            });
        };

        sweep::delete_chapter(&mut context.state, chapter_id);

        accept(())
    }
}

impl Step<SweepSubtree<'_>, MockContext> for Mock {
    // Required Orchestra execution level.
    type Level = ReptRead;
    // Shared repository error type.
    type Error = BaseError;

    // Execute the repository operation.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &SweepSubtree<'_>,
    ) -> BaseRest<()> {
        //
        sweep::sweep_target(&mut context.state, oper.target);

        accept(())
    }
}
