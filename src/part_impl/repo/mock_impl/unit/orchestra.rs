use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::read::proj::unit::{UnitCounters, UnitInfo, UnitOrder};
use crate::part::repo::oper::unit::{
    ApplyUnitEdits, ListUnitInfos, ListUnitOrders,
};
use crate::part_impl::repo::mock_impl::unit::{
    apply_edits, list_infos, list_positions,
};
use crate::part_impl::repo::mock_impl::{Mock, MockContext};
use crate::result::{BaseError, BaseResult};

impl Run<ListUnitInfos<'_>> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &ListUnitInfos<'_>) -> BaseResult<Vec<UnitInfo>> {
        //
        let state = self.state.lock().unwrap();

        list_infos(&state, oper.page_id)
    }
}

impl Step<ListUnitOrders<'_>, MockContext> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListUnitOrders<'_>,
    ) -> BaseResult<Vec<UnitOrder>> {
        list_positions(&context.state, oper.page_id)
    }
}

impl Step<ApplyUnitEdits<'_>, MockContext> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ApplyUnitEdits<'_>,
    ) -> BaseResult<UnitCounters> {
        apply_edits(&mut context.state, oper.page_id, oper.orders, oper.edits)
    }
}
