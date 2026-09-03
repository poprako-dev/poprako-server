//! Diesel-backed hierarchy mark-and-sweep operations.

/// Locks active roots and atomically marks aggregate descendants.
// Active hierarchy marking operations.
mod mark;
/// Claims tombstones and physically removes their direct dependants.
// Physical hierarchy sweeping operations.
mod sweep;

/// PostgreSQL mark-and-sweep integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

use poprako_orchestra::{AtLeast, Level, Step};
use tracing::instrument;

use crate::model::read::proj::subtree_delete::{
    SubtreeDeleteScope, SubtreeDeleteSweepTarget,
};
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::subtree_delete::{
    ClaimSubtreeSweep, DeleteSubtree, ListSubtreePageIds,
    LockSubtreeDeleteScope, MarkSubtree, SweepSubtree,
};
use crate::part_impl::repo::HybRepo;
use crate::result::{BaseError, BaseRest};
use crate::shared::RdbContext;

impl<L> Step<LockSubtreeDeleteScope<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Required Orchestra execution level.
    type Level = ReptRead;
    // Shared repository error type.
    type Error = BaseError;

    // Execute the repository operation.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &LockSubtreeDeleteScope<'_>,
    ) -> BaseRest<SubtreeDeleteScope> {
        mark::lock_scope(context.conn(), &oper.root).await
    }
}

impl<L> Step<MarkSubtree<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Required Orchestra execution level.
    type Level = ReptRead;
    // Shared repository error type.
    type Error = BaseError;

    // Execute the repository operation.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &MarkSubtree<'_>,
    ) -> BaseRest<()> {
        mark::mark_scope(context.conn(), oper.scope).await
    }
}

impl<L> Step<ClaimSubtreeSweep, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Required Orchestra execution level.
    type Level = ReptRead;
    // Shared repository error type.
    type Error = BaseError;

    // Execute the repository operation.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        _oper: &ClaimSubtreeSweep,
    ) -> BaseRest<Option<SubtreeDeleteSweepTarget>> {
        sweep::claim(context.conn()).await
    }
}

impl<L> Step<ListSubtreePageIds<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Required Orchestra execution level.
    type Level = ReptRead;
    // Shared repository error type.
    type Error = BaseError;

    // Execute the repository operation.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ListSubtreePageIds<'_>,
    ) -> BaseRest<Vec<String>> {
        sweep::list_page_ids(context.conn(), oper.chapter_id).await
    }
}

impl<L> Step<DeleteSubtree<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Required Orchestra execution level.
    type Level = ReptRead;
    // Shared repository error type.
    type Error = BaseError;

    // Execute the repository operation.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &DeleteSubtree<'_>,
    ) -> BaseRest<()> {
        sweep::delete_active_scope(context.conn(), oper.scope).await
    }
}

impl<L> Step<SweepSubtree<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Required Orchestra execution level.
    type Level = ReptRead;
    // Shared repository error type.
    type Error = BaseError;

    // Execute the repository operation.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &SweepSubtree<'_>,
    ) -> BaseRest<()> {
        sweep::delete_target(context.conn(), oper.target).await
    }
}
