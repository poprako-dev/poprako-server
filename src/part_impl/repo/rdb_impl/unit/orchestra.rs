use poprako_orchestra::{Run, Step};

use crate::model::unit::UnitCounters;
use crate::model::unit::UnitIndex;
use crate::model::unit::UnitInfo;
use crate::part::repo::oper::unit::{
    CountUnits, CreateUnit, DeleteUnit, ListUnitIndexes, ListUnitInfos,
    SaveUnit, UpdateUnitIndexes,
};
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::repo::rdb_impl::unit::{
    count_by_page_id, create_unit, delete_by_id_in_page,
    list_all_infos_by_page_id, list_indexes_by_page_id, list_infos_by_page_id,
    save_unit, update_indexes_by_page_id,
};
use crate::part_impl::shared::RdbContext;
use crate::result::{RegularError, RegularResult};

impl<'a> Run<ListUnitInfos<'a>> for RdbRepo {
    type Error = RegularError;
    async fn run(
        &self,
        oper: &ListUnitInfos<'a>,
    ) -> RegularResult<Vec<UnitInfo>> {
        match oper {
            ListUnitInfos::Page { page_id, page } => {
                submit_query!(self.core, list_infos_by_page_id, page_id, *page)
            }
            ListUnitInfos::AllPage { page_id } => {
                submit_query!(self.core, list_all_infos_by_page_id, page_id)
            }
        }
    }
}

impl<'a> Step<ListUnitInfos<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListUnitInfos<'a>,
    ) -> RegularResult<Vec<UnitInfo>> {
        match oper {
            ListUnitInfos::Page { page_id, page } => {
                list_infos_by_page_id(context.conn(), page_id, *page).await
            }
            ListUnitInfos::AllPage { page_id } => {
                list_all_infos_by_page_id(context.conn(), page_id).await
            }
        }
    }
}

impl<'a> Step<CreateUnit<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateUnit<'a>,
    ) -> RegularResult<()> {
        create_unit(context.conn(), oper.page_id, oper.id, oper.payload).await
    }
}
impl<'a> Step<SaveUnit<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &SaveUnit<'a>,
    ) -> RegularResult<()> {
        save_unit(context.conn(), oper.page_id, oper.id, oper.payload).await
    }
}
impl<'a> Step<DeleteUnit<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteUnit<'a>,
    ) -> RegularResult<()> {
        delete_by_id_in_page(context.conn(), oper.page_id, oper.id).await
    }
}
impl<'a> Step<ListUnitIndexes<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListUnitIndexes<'a>,
    ) -> RegularResult<Vec<UnitIndex>> {
        list_indexes_by_page_id(context.conn(), oper.page_id).await
    }
}
impl<'a> Step<UpdateUnitIndexes<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UpdateUnitIndexes<'a>,
    ) -> RegularResult<()> {
        update_indexes_by_page_id(context.conn(), oper.page_id, oper.updates)
            .await
    }
}
impl<'a> Step<CountUnits<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CountUnits<'a>,
    ) -> RegularResult<UnitCounters> {
        count_by_page_id(context.conn(), oper.page_id).await
    }
}
