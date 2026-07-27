use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::read::proj::unit::{UnitCounters, UnitInfo, UnitOrder};
use crate::part::repo::oper::unit::{
    ApplyUnitEdits, ListUnitInfos, ListUnitOrders,
};
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::repo::rdb_impl::unit::step_impl::{
    apply_edits, list_infos, list_orders_for_update,
};
use crate::part_impl::shared::RdbContext;
use crate::result::{BaseError, BaseResult};

impl Run<ListUnitInfos<'_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &ListUnitInfos<'_>) -> BaseResult<Vec<UnitInfo>> {
        submit_query!(self.core, list_infos, oper.page_id)
    }
}

impl Step<ListUnitOrders<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListUnitOrders<'_>,
    ) -> BaseResult<Vec<UnitOrder>> {
        list_orders_for_update(context.conn(), oper.page_id).await
    }
}

impl Step<ApplyUnitEdits<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ApplyUnitEdits<'_>,
    ) -> BaseResult<UnitCounters> {
        apply_edits(context.conn(), oper.page_id, oper.orders, oper.edits).await
    }
}
