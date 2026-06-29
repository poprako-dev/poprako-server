//! Mock implementations of `UnitRepo` and `UnitRepoTransactional`.

use std::collections::HashSet;

use async_trait::async_trait;

use poprako_transactional::advance::Advance;

use crate::model::unit::{UnitCounters, UnitInfo};
use crate::part::repo::step::unit::{CountByPage, ListInfosByPage, ReplaceInfosByPage};
use crate::part::repo::unit::{UnitRepo, UnitRepoTransactional};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo_mock::{Mock, MockContext, MockState, MockTransactional, expected, now};
use crate::result::RootError;

impl UnitRepo<MockContext> for Mock {}

impl UnitRepoTransactional<MockContext> for MockTransactional {}

fn list_units(state: &MockState, page_id: &str) -> Vec<UnitInfo> {
    let mut unit_infos = state
        .units
        .iter()
        .filter(|unit_info| unit_info.page_id == page_id)
        .cloned()
        .collect::<Vec<_>>();
    unit_infos.sort_by(|left, right| left.index.cmp(&right.index));

    unit_infos
}

fn count_units(state: &MockState, page_id: &str) -> UnitCounters {
    let unit_infos = list_units(state, page_id);

    UnitCounters {
        total_unit_count: unit_infos.len() as i32,
        translated_unit_count: unit_infos
            .iter()
            .filter(|unit_info| unit_info.is_translated())
            .count() as i32,
        proofread_unit_count: unit_infos
            .iter()
            .filter(|unit_info| unit_info.is_proofread)
            .count() as i32,
    }
}

#[async_trait]
impl<'a> Execute<ListInfosByPage<'a>> for Mock {
    type Error = RootError;

    async fn execute(&self, step: &ListInfosByPage<'a>) -> Result<Vec<UnitInfo>, Self::Error> {
        let state = self.state.lock().unwrap();

        Ok(list_units(&state, step.page_id))
    }
}

#[async_trait]
impl<'a> Advance<ListInfosByPage<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &ListInfosByPage<'a>,
    ) -> Result<Vec<UnitInfo>, Self::Error> {
        Ok(list_units(&context.state, step.page_id))
    }
}

#[async_trait]
impl<'a> Advance<ReplaceInfosByPage<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &ReplaceInfosByPage<'a>,
    ) -> Result<(), Self::Error> {
        let mut seen_ids = HashSet::new();

        for unit_info in step.unit_infos {
            if unit_info.page_id != step.page_id {
                return Err(expected("error-invalid-unit-operation"));
            }

            if !seen_ids.insert(unit_info.id.clone()) {
                return Err(expected("error-unit-duplicate"));
            }

            if context.state.units.iter().any(|current_unit_info| {
                current_unit_info.page_id != step.page_id && current_unit_info.id == unit_info.id
            }) {
                return Err(expected("error-unit-duplicate"));
            }
        }

        context
            .state
            .units
            .retain(|unit_info| unit_info.page_id != step.page_id);

        let mut unit_infos = step.unit_infos.to_vec();
        for unit_info in &mut unit_infos {
            unit_info.updated_at = now();
        }
        context.state.units.extend(unit_infos);

        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<CountByPage<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &CountByPage<'a>,
    ) -> Result<UnitCounters, Self::Error> {
        Ok(count_units(&context.state, step.page_id))
    }
}
