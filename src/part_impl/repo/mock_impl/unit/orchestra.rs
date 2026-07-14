use poprako_orchestra::{Run, Step};

use tracing::instrument;

use crate::model::unit::{UnitCounters, UnitIndex, UnitInfo};
use crate::part::repo::oper::unit::{
    CountUnits, CreateUnit, DeleteUnit, ListUnitIndexes, ListUnitInfos,
    SaveUnit, UpdateUnitIndexes,
};
use crate::part_impl::repo::mock_impl::unit::{
    count_units, create_unit, list_all_units, list_units, save_unit,
};
use crate::part_impl::repo::mock_impl::{Mock, MockContext, expected, now};
use crate::result::{RegularError, RegularResult};

impl<'a> Run<ListUnitInfos<'a>> for Mock {
    type Error = RegularError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListUnitInfos<'a>,
    ) -> RegularResult<Vec<UnitInfo>> {
        //
        let state = self.state.lock().unwrap();

        match oper {
            //
            ListUnitInfos::Page { page_id, page } => {
                Ok(list_units(&state, page_id, *page))
            }

            ListUnitInfos::AllPage { page_id } => {
                Ok(list_all_units(&state, page_id))
            }
        }
    }
}
impl<'a> Step<ListUnitInfos<'a>, MockContext> for Mock {
    type Error = RegularError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListUnitInfos<'a>,
    ) -> RegularResult<Vec<UnitInfo>> {
        match oper {
            //
            ListUnitInfos::Page { page_id, page } => {
                Ok(list_units(&context.state, page_id, *page))
            }

            ListUnitInfos::AllPage { page_id } => {
                Ok(list_all_units(&context.state, page_id))
            }
        }
    }
}
impl<'a> Step<CreateUnit<'a>, MockContext> for Mock {
    type Error = RegularError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CreateUnit<'a>,
    ) -> RegularResult<()> {
        create_unit(&mut context.state, oper.page_id, oper.id, oper.payload)
    }
}
impl<'a> Step<SaveUnit<'a>, MockContext> for Mock {
    type Error = RegularError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &SaveUnit<'a>,
    ) -> RegularResult<()> {
        save_unit(&mut context.state, oper.page_id, oper.id, oper.payload)
    }
}
impl<'a> Step<DeleteUnit<'a>, MockContext> for Mock {
    type Error = RegularError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeleteUnit<'a>,
    ) -> RegularResult<()> {
        //
        context.state.units.retain(|info| {
            !(info.page_id == oper.page_id && info.id == oper.id)
        });

        Ok(())
    }
}
impl<'a> Step<ListUnitIndexes<'a>, MockContext> for Mock {
    type Error = RegularError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListUnitIndexes<'a>,
    ) -> RegularResult<Vec<UnitIndex>> {
        Ok(context
            .state
            .units
            .iter()
            .filter(|info| info.page_id == oper.page_id)
            .map(|info| UnitIndex {
                id: info.id.clone(),
                index: info.index,
            })
            .collect())
    }
}
impl<'a> Step<UpdateUnitIndexes<'a>, MockContext> for Mock {
    type Error = RegularError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UpdateUnitIndexes<'a>,
    ) -> RegularResult<()> {
        //
        for update in oper.updates {
            //
            let info = context
                .state
                .units
                .iter_mut()
                .find(|info| {
                    info.page_id == oper.page_id && info.id == update.id
                })
                .ok_or_else(|| expected("error-unit-not-found"))?;

            info.index = update.index;

            info.updated_at = now();
        }

        Ok(())
    }
}
impl<'a> Step<CountUnits<'a>, MockContext> for Mock {
    type Error = RegularError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CountUnits<'a>,
    ) -> RegularResult<UnitCounters> {
        Ok(count_units(&context.state, oper.page_id))
    }
}
