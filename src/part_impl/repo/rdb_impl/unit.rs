//! RDB-backed unit repository.

// Transaction-scoped unit operations.
mod step_impl;

/// Unit RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

use poprako_orchestra::{AtLeast, Level, Run, Step};
use tracing::instrument;

use crate::model::read::proj::unit::{UnitCounters, UnitInfo, UnitOrder};
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::unit::{
    ApplyUnitEdits, ListUnitInfos, ListUnitOrders,
};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::unit::step_impl::{
    apply_edits, list_infos, list_orders_for_update,
};
use crate::result::{BaseError, BaseRest};
use crate::shared::RdbContext;

impl Run<ListUnitInfos<'_>> for HybRepo {
    // Error type for the Run trait impl on unit list query.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Lists visible Units in verified linked-list order for the given page.
    #[instrument(level = "info", skip_all)]
    async fn run(&self, oper: &ListUnitInfos<'_>) -> BaseRest<Vec<UnitInfo>> {
        submit_query!(self.core, list_infos, oper.page_id)
    }
}

impl<L> Step<ListUnitOrders<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Error type for the Step trait impl on unit order list.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Locks and lists the complete Unit chain, including tombstones, within a transaction.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ListUnitOrders<'_>,
    ) -> BaseRest<Vec<UnitOrder>> {
        list_orders_for_update(context.conn(), oper.page_id).await
    }
}

impl<L> Step<ApplyUnitEdits<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Error type for the Step trait impl on unit edit application.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Applies normalized Unit edits and returns the latest visible counters within a transaction.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ApplyUnitEdits<'_>,
    ) -> BaseRest<UnitCounters> {
        apply_edits(context.conn(), oper.page_id, oper.orders, oper.edits).await
    }
}
