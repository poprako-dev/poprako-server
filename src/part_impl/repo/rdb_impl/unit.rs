//! RDB-backed unit repository.

// Unit edit application and sequence mutation.
mod edit;
// Unit sequence reads and chain validation.
mod sequence;

/// Unit RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

use poprako_orchestra::{AtLeast, Level, Run, Step};
use tracing::instrument;

use crate::model::read::proj::unit::{UnitCountMetrics, UnitInfo, UnitOrder};
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::unit::{
    ApplyUnitEdits, ListUnitInfosByIds, ListUnitInfosByPageIds,
    ListUnitInfosInChapterOrder, ListUnitOrders, SearchChapterUnitIds,
};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::unit::edit::apply_edits;
use crate::part_impl::repo::rdb_impl::unit::sequence::{
    list_infos_by_ids, list_infos_by_page_ids, list_infos_in_chapter_order,
    list_orders, search_chapter_ids,
};
use crate::result::{BaseError, BaseRest};
use crate::shared::RdbContext;

impl Run<ListUnitInfosByPageIds<'_>> for HybRepo {
    // Error type for the Run trait impl on the page-id unit list query.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Lists visible Units for the requested page ids.
    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &ListUnitInfosByPageIds<'_>,
    ) -> BaseRest<Vec<UnitInfo>> {
        submit_query!(self.core, list_infos_by_page_ids, oper.page_ids)
    }
}

impl<L> Step<ListUnitInfosByIds<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Error type for the transaction-scoped Unit selection query.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Lists every requested Unit that currently exists in the transaction.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ListUnitInfosByIds<'_>,
    ) -> BaseRest<Vec<UnitInfo>> {
        list_infos_by_ids(context.conn(), oper.ids).await
    }
}

impl<L> Step<SearchChapterUnitIds<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // The minimum transaction level needed by the search snapshot.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Searches only visible Unit IDs for overflow-aware Chapter text matching.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &SearchChapterUnitIds<'_>,
    ) -> BaseRest<Vec<String>> {
        //
        search_chapter_ids(
            context.conn(),
            oper.chapter_id,
            oper.part,
            oper.phrase,
            oper.fetch_count,
        )
        .await
    }
}

impl<L> Step<ListUnitInfosInChapterOrder<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // The minimum transaction level needed by the ordered search snapshot.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Loads selected Units in their stable Chapter presentation order.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ListUnitInfosInChapterOrder<'_>,
    ) -> BaseRest<Vec<UnitInfo>> {
        list_infos_in_chapter_order(context.conn(), oper.ids).await
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

    // Lists the complete Unit chain, including tombstones, within a transaction.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ListUnitOrders<'_>,
    ) -> BaseRest<Vec<UnitOrder>> {
        list_orders(context.conn(), oper.page_id).await
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
    ) -> BaseRest<UnitCountMetrics> {
        apply_edits(context.conn(), oper.page_id, oper.orders, oper.edits).await
    }
}
