use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::read::proj::unit::{UnitCountMetrics, UnitInfo, UnitOrder};
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::unit::{
    ApplyUnitEdits, ListUnitInfos, ListUnitInfosByIds, ListUnitInfosByPageIds,
    ListUnitOrders,
};
use crate::part_impl::repo::mock_impl::unit::{
    apply_edits, list_infos, list_infos_by_ids, list_infos_by_page_ids,
    list_orders,
};
use crate::part_impl::repo::mock_impl::{Mock, MockContext};
use crate::result::{BaseError, BaseRest, accept};

impl Run<ListUnitInfos<'_>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(&self, oper: &ListUnitInfos<'_>) -> BaseRest<Vec<UnitInfo>> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        list_infos(&state, oper.page_id)
    }
}

impl Run<ListUnitInfosByPageIds<'_>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(
        &self,
        oper: &ListUnitInfosByPageIds<'_>,
    ) -> BaseRest<Vec<UnitInfo>> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        list_infos_by_page_ids(&state, oper.page_ids)
    }
}

impl Step<ListUnitInfosByIds<'_>, MockContext> for Mock {
    // Internal type alias for `Level`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Lists every requested Unit that exists in the transaction snapshot.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListUnitInfosByIds<'_>,
    ) -> BaseRest<Vec<UnitInfo>> {
        accept(list_infos_by_ids(&context.state, oper.ids))
    }
}

impl Step<ListUnitOrders<'_>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListUnitOrders<'_>,
    ) -> BaseRest<Vec<UnitOrder>> {
        list_orders(&context.state, oper.page_id)
    }
}

impl Step<ApplyUnitEdits<'_>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ApplyUnitEdits<'_>,
    ) -> BaseRest<UnitCountMetrics> {
        apply_edits(&mut context.state, oper.page_id, oper.orders, oper.edits)
    }
}
