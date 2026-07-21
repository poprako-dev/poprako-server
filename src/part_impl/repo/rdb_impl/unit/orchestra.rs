use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::unit::{UnitCounters, UnitIndex, UnitInfo};
use crate::part::repo::oper::unit::{
    CountUnits, CreateUnit, DeleteUnit, ListUnitIndexes, ListUnitInfos,
    SaveUnit, UpdateUnitIndexes,
};
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::repo::rdb_impl::unit::step_impl::{
    count_by_page_id, create_unit, delete_by_id_in_page,
    list_all_infos_by_page_id, list_indexes_by_page_id, list_infos_by_page_id,
    save_unit, update_indexes_by_page_id,
};
use crate::part_impl::shared::RdbContext;
use crate::result::{BaseError, BaseResult};

impl Run<ListUnitInfos<'_>> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &ListUnitInfos<'_>) -> BaseResult<Vec<UnitInfo>> {
        match oper {
            //
            ListUnitInfos::Page { page_id, page } => {
                submit_query!(self.core, list_infos_by_page_id, page_id, *page)
            }

            ListUnitInfos::AllPage { page_id } => {
                submit_query!(self.core, list_all_infos_by_page_id, page_id)
            }
        }
    }
}

impl Step<ListUnitInfos<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListUnitInfos<'_>,
    ) -> BaseResult<Vec<UnitInfo>> {
        match oper {
            //
            ListUnitInfos::Page { page_id, page } => {
                list_infos_by_page_id(context.conn(), page_id, *page).await
            }

            ListUnitInfos::AllPage { page_id } => {
                list_all_infos_by_page_id(context.conn(), page_id).await
            }
        }
    }
}

impl Step<CreateUnit<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateUnit<'_>,
    ) -> BaseResult<()> {
        create_unit(context.conn(), oper.page_id, oper.id, oper.payload).await
    }
}
impl Step<SaveUnit<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &SaveUnit<'_>,
    ) -> BaseResult<()> {
        save_unit(context.conn(), oper.page_id, oper.id, oper.payload).await
    }
}
impl Step<DeleteUnit<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteUnit<'_>,
    ) -> BaseResult<()> {
        delete_by_id_in_page(context.conn(), oper.page_id, oper.id).await
    }
}
impl Step<ListUnitIndexes<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListUnitIndexes<'_>,
    ) -> BaseResult<Vec<UnitIndex>> {
        list_indexes_by_page_id(context.conn(), oper.page_id).await
    }
}
impl Step<UpdateUnitIndexes<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UpdateUnitIndexes<'_>,
    ) -> BaseResult<()> {
        update_indexes_by_page_id(context.conn(), oper.page_id, oper.updates)
            .await
    }
}
impl Step<CountUnits<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CountUnits<'_>,
    ) -> BaseResult<UnitCounters> {
        count_by_page_id(context.conn(), oper.page_id).await
    }
}
