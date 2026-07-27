use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::read::proj::unit::{UnitCounters, UnitInfo, UnitOrder};
use crate::part::repo::oper::unit::{
    ApplyUnitEdits, ListUnitInfos, ListUnitOrders,
};
use crate::part_impl::repo::mock_impl::unit::{
    apply_edits, list_infos, list_orders,
};
use crate::part_impl::repo::mock_impl::{Mock, MockContext};
use crate::result::{BaseError, BaseRest};

impl Run<ListUnitInfos<'_>> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `run`.
    async fn run(&self, oper: &ListUnitInfos<'_>) -> BaseRest<Vec<UnitInfo>> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        list_infos(&state, oper.page_id)
    }
}

impl Step<ListUnitOrders<'_>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
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
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ApplyUnitEdits<'_>,
    ) -> BaseRest<UnitCounters> {
        apply_edits(&mut context.state, oper.page_id, oper.orders, oper.edits)
    }
}
