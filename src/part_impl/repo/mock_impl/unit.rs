//! Mock implementation of `UnitRepo`.

use crate::model::unit::{UnitContent, UnitCounters, UnitInfo};
use crate::part_impl::repo::mock_impl::{
    MockState, expected, now,
};
use crate::result::{BaseResult, accept};

mod orchestra;

#[cfg(test)]
mod tests;

fn list_all_units(state: &MockState, page_id: &str) -> Vec<UnitInfo> {
    //
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
            //
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

fn write_payload(unit_info: &mut UnitInfo, payload: &UnitContent) {
    //
    unit_info.is_bubble = payload.is_bubble;

    unit_info.is_proofread = payload.is_proofread;

    unit_info.x_coord = payload.x_coord;

    unit_info.y_coord = payload.y_coord;

    unit_info.translated_text = payload.translated_text.clone();

    unit_info.last_translator_id = payload.last_translator_id.clone();

    unit_info.proofread_text = payload.proofread_text.clone();

    unit_info.last_proofreader_id = payload.last_proofreader_id.clone();

    unit_info.updated_at = now();
}

fn unit_from_payload(
    page_id: &str,
    id: &str,
    index: i32,
    payload: &UnitContent,
) -> UnitInfo {
    //
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
        last_translator_id: payload.last_translator_id.clone(),
        proofread_text: payload.proofread_text.clone(),
        last_proofreader_id: payload.last_proofreader_id.clone(),
        created_at: time,
        updated_at: time,
    }
}

fn create_unit(
    state: &mut MockState,
    page_id: &str,
    id: &str,
    payload: &UnitContent,
) -> BaseResult<()> {
    //
    if state.units.iter().any(|unit_info| unit_info.id == id) {
        return Err(expected("error-unit-duplicate"));
    }

    let index = next_index(state, page_id);

    let unit_info = unit_from_payload(page_id, id, index, payload);

    state.units.push(unit_info);

    accept(())
}

fn save_unit(
    state: &mut MockState,
    page_id: &str,
    id: &str,
    payload: &UnitContent,
) -> BaseResult<()> {
    //
    let existing_position =
        state.units.iter().position(|unit_info| unit_info.id == id);

    let Some(existing_position) = existing_position else {
        return create_unit(state, page_id, id, payload);
    };

    if state.units[existing_position].page_id != page_id {
        return Err(expected("error-unit-duplicate"));
    }

    write_payload(&mut state.units[existing_position], payload);

    accept(())
}
