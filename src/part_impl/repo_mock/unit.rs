//! Mock implementations of `UnitRepo` and `UnitRepoTransactional`.

use async_trait::async_trait;

use poprako_transactional::advance::Advance;

use crate::model::unit::{UnitCounters, UnitIndex, UnitInfo, UnitOper, UnitPayload};
use crate::part::repo::step::unit::{
    CountByPageId, CreateInfo, DeleteByIdInPage, ListIndexesByPageId, ListInfosByPageId, SaveInfo,
    UpdateIndexesByPageId,
};
use crate::part::repo::unit::{UnitRepo, UnitRepoTransactional};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo_mock::{Mock, MockContext, MockState, MockTransactional, expected, now};
use crate::result::{RegularError, RegularResult};

impl UnitRepo<MockContext> for Mock {}

impl UnitRepoTransactional<MockContext> for MockTransactional {}

fn list_units(state: &MockState, page_id: &str) -> Vec<UnitInfo> {
    let mut unit_infos = state
        .units
        .iter()
        .filter(|unit_info| unit_info.page_id == page_id)
        .cloned()
        .collect::<Vec<_>>();

    unit_infos.sort_by(|left, right| {
        left.index
            .cmp(&right.index)
            .then_with(|| left.id.cmp(&right.id))
    });

    unit_infos
}

fn count_units(state: &MockState, page_id: &str) -> UnitCounters {
    state
        .units
        .iter()
        .filter(|unit_info| unit_info.page_id == page_id)
        .fold(UnitCounters::default(), |mut counters, unit_info| {
            counters.total_unit_count += 1;

            if unit_info.is_translated() {
                counters.translated_unit_count += 1;
            }

            if unit_info.is_proofread {
                counters.proofread_unit_count += 1;
            }

            counters
        })
}

fn next_index(state: &MockState, page_id: &str) -> i32 {
    state
        .units
        .iter()
        .filter(|unit_info| unit_info.page_id == page_id)
        .map(|unit_info| unit_info.index)
        .max()
        .map(|index| index + 1)
        .unwrap_or(0)
}

fn write_payload(unit_info: &mut UnitInfo, payload: &UnitPayload) {
    unit_info.is_bubble = payload.is_bubble;
    unit_info.is_proofread = payload.is_proofread;
    unit_info.x_coord = payload.x_coord;
    unit_info.y_coord = payload.y_coord;
    unit_info.translated_text = payload.translated_text.clone();
    unit_info.translator_comment = payload.translator_comment.clone();
    unit_info.last_translator_id = payload.last_translator_id.clone();
    unit_info.proofread_text = payload.proofread_text.clone();
    unit_info.proofreader_comment = payload.proofreader_comment.clone();
    unit_info.last_proofreader_id = payload.last_proofreader_id.clone();
    unit_info.updated_at = now();
}

fn unit_from_payload(page_id: &str, id: &str, index: i32, payload: &UnitPayload) -> UnitInfo {
    let time = now();

    UnitInfo {
        id: id.into(),
        page_id: page_id.into(),
        index,
        is_bubble: payload.is_bubble,
        is_proofread: payload.is_proofread,
        x_coord: payload.x_coord,
        y_coord: payload.y_coord,
        translated_text: payload.translated_text.clone(),
        translator_comment: payload.translator_comment.clone(),
        last_translator_id: payload.last_translator_id.clone(),
        proofread_text: payload.proofread_text.clone(),
        proofreader_comment: payload.proofreader_comment.clone(),
        last_proofreader_id: payload.last_proofreader_id.clone(),
        created_at: time,
        updated_at: time,
    }
}

fn create_unit(
    state: &mut MockState,
    page_id: &str,
    id: &str,
    payload: &UnitPayload,
) -> RegularResult<()> {
    if state.units.iter().any(|unit_info| unit_info.id == id) {
        return Err(expected("error-unit-duplicate"));
    }

    let index = next_index(state, page_id);
    let unit_info = unit_from_payload(page_id, id, index, payload);

    state.units.push(unit_info);

    Ok(())
}

fn save_unit(
    state: &mut MockState,
    page_id: &str,
    id: &str,
    payload: &UnitPayload,
) -> RegularResult<()> {
    let existing_position = state.units.iter().position(|unit_info| unit_info.id == id);

    let Some(existing_position) = existing_position else {
        return create_unit(state, page_id, id, payload);
    };

    if state.units[existing_position].page_id != page_id {
        return Err(expected("error-unit-duplicate"));
    }

    write_payload(&mut state.units[existing_position], payload);

    Ok(())
}

#[async_trait]
impl<'a> Execute<ListInfosByPageId<'a>> for Mock {
    type Error = RegularError;

    async fn execute(&self, step: &ListInfosByPageId<'a>) -> Result<Vec<UnitInfo>, Self::Error> {
        let state = self.state.lock().unwrap();

        Ok(list_units(&state, step.page_id))
    }
}

#[async_trait]
impl<'a> Advance<ListInfosByPageId<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &ListInfosByPageId<'a>,
    ) -> Result<Vec<UnitInfo>, Self::Error> {
        Ok(list_units(&context.state, step.page_id))
    }
}

#[async_trait]
impl<'a> Advance<CreateInfo<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &CreateInfo<'a>,
    ) -> Result<(), Self::Error> {
        let UnitOper::Create {
            id: Some(id),
            payload,
            ..
        } = step.oper
        else {
            return Err(expected("error-invalid-unit-oper"));
        };

        create_unit(&mut context.state, step.page_id, id, payload)
    }
}

#[async_trait]
impl<'a> Advance<SaveInfo<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &SaveInfo<'a>,
    ) -> Result<(), Self::Error> {
        let UnitOper::Save { id, payload } = step.oper else {
            return Err(expected("error-invalid-unit-oper"));
        };

        save_unit(&mut context.state, step.page_id, id, payload)
    }
}

#[async_trait]
impl<'a> Advance<DeleteByIdInPage<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &DeleteByIdInPage<'a>,
    ) -> Result<(), Self::Error> {
        context
            .state
            .units
            .retain(|unit_info| !(unit_info.page_id == step.page_id && unit_info.id == step.id));

        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<ListIndexesByPageId<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &ListIndexesByPageId<'a>,
    ) -> Result<Vec<UnitIndex>, Self::Error> {
        Ok(context
            .state
            .units
            .iter()
            .filter(|unit_info| unit_info.page_id == step.page_id)
            .map(|unit_info| UnitIndex {
                id: unit_info.id.clone(),
                index: unit_info.index,
            })
            .collect())
    }
}

#[async_trait]
impl<'a> Advance<UpdateIndexesByPageId<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &UpdateIndexesByPageId<'a>,
    ) -> Result<(), Self::Error> {
        for unit_index_update in step.updates {
            let unit_info = context
                .state
                .units
                .iter_mut()
                .find(|unit_info| {
                    unit_info.page_id == step.page_id && unit_info.id == unit_index_update.id
                })
                .ok_or_else(|| expected("error-unit-not-found"))?;

            unit_info.index = unit_index_update.index;

            unit_info.updated_at = now();
        }

        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<CountByPageId<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &CountByPageId<'a>,
    ) -> Result<UnitCounters, Self::Error> {
        Ok(count_units(&context.state, step.page_id))
    }
}
