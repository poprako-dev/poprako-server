//! In-memory implementation of Unit repository operations.

use crate::model::read::proj::unit::{UnitCounters, UnitInfo, UnitOrder};
use crate::model::write::unit::UnitEdit;
use crate::part_impl::repo::mock_impl::{
    MockState, expected, now, unrecoverable,
};
use crate::result::{BaseResult, accept};
use crate::util::PatchField;

mod orchestra;

#[cfg(test)]
mod tests;

fn list_infos(state: &MockState, page_id: &str) -> BaseResult<Vec<UnitInfo>> {
    //
    let mut unit_infos = state
        .units
        .iter()
        .filter(|unit_info| unit_info.page_id == page_id)
        .cloned()
        .collect::<Vec<_>>();

    order_units(
        &mut unit_infos,
        |unit_info| unit_info.id.as_str(),
        |unit_info| unit_info.next_id.as_deref(),
    )?;

    unit_infos.retain(|unit_info| unit_info.hidden_at.is_none());

    accept(unit_infos)
}

fn list_positions(
    state: &MockState,
    page_id: &str,
) -> BaseResult<Vec<UnitOrder>> {
    //
    let mut unit_orders = state
        .units
        .iter()
        .filter(|unit_info| unit_info.page_id == page_id)
        .map(|unit_info| UnitOrder {
            id: unit_info.id.clone(),
            next_id: unit_info.next_id.clone(),
            is_hidden: unit_info.hidden_at.is_some(),
        })
        .collect::<Vec<_>>();

    order_units(
        &mut unit_orders,
        |unit_order| unit_order.id.as_str(),
        |unit_order| unit_order.next_id.as_deref(),
    )?;

    accept(unit_orders)
}

fn apply_edits(
    state: &mut MockState,
    page_id: &str,
    orders: &[UnitOrder],
    edits: &[UnitEdit],
) -> BaseResult<UnitCounters> {
    //
    for edit in edits {
        match edit {
            //
            UnitEdit::Delete { id } => {
                //
                let unit_info = find_unit_mut(state, page_id, id)?;

                unit_info.hidden_at = Some(now());

                unit_info.updated_at = now();
            }

            UnitEdit::Save { id, .. } => {
                save_edit(state, page_id, id, edit)?;
            }
        }
    }

    for order in orders {
        //
        let unit_info = find_unit_mut(state, page_id, &order.id)?;

        unit_info.next_id = order.next_id.clone();

        unit_info.hidden_at = match order.is_hidden {
            //
            true => unit_info.hidden_at.or_else(|| Some(now())),

            false => None,
        };

        unit_info.updated_at = now();
    }

    let unit_infos = list_infos(state, page_id)?;

    accept(count_infos(&unit_infos))
}

fn save_edit(
    state: &mut MockState,
    page_id: &str,
    id: &str,
    edit: &UnitEdit,
) -> BaseResult<()> {
    //
    let position = state.units.iter().position(|unit_info| unit_info.id == id);

    let Some(position) = position else {
        //
        let unit_info = unit_from_edit(page_id, id, edit)?;

        state.units.push(unit_info);

        return accept(());
    };

    if state.units[position].page_id != page_id {
        return Err(expected("error-unit-duplicate"));
    }

    write_edit(&mut state.units[position], edit);

    accept(())
}

fn unit_from_edit(
    page_id: &str,
    id: &str,
    edit: &UnitEdit,
) -> BaseResult<UnitInfo> {
    //
    let UnitEdit::Save {
        is_bubble: Some(is_bubble),
        coord: Some(coord),
        translation,
        revision,
        ..
    } = edit
    else {
        return Err(expected("error-invalid-unit-oper"));
    };

    let (translated_text, last_translator_id) = match translation {
        //
        PatchField::Assign(translation) => (
            Some(translation.translated_text.clone()),
            Some(translation.last_translator_id.clone()),
        ),

        PatchField::Clear | PatchField::Skip => (None, None),
    };

    let (is_proofread, proofread_text, last_proofreader_id) = match revision {
        //
        PatchField::Assign(revision) => (
            revision.is_proofread,
            revision.proofread_text.clone(),
            Some(revision.last_proofreader_id.clone()),
        ),

        PatchField::Clear | PatchField::Skip => (false, None, None),
    };

    let current_time = now();

    accept(UnitInfo {
        id: id.to_string(),
        page_id: page_id.to_string(),
        next_id: None,
        is_bubble: *is_bubble,
        coord: coord.clone(),
        translated_text,
        last_translator_id,
        is_proofread,
        proofread_text,
        last_proofreader_id,
        hidden_at: None,
        created_at: current_time,
        updated_at: current_time,
    })
}

fn write_edit(unit_info: &mut UnitInfo, edit: &UnitEdit) {
    //
    let UnitEdit::Save {
        is_bubble,
        coord,
        translation,
        revision,
        ..
    } = edit
    else {
        return;
    };

    unit_info.hidden_at = None;

    if let Some(is_bubble) = is_bubble {
        unit_info.is_bubble = *is_bubble;
    }

    if let Some(coord) = coord {
        unit_info.coord = coord.clone();
    }

    match translation {
        //
        PatchField::Clear => {
            //
            unit_info.translated_text = None;

            unit_info.last_translator_id = None;
        }

        PatchField::Assign(translation) => {
            //
            unit_info.translated_text =
                Some(translation.translated_text.clone());

            unit_info.last_translator_id =
                Some(translation.last_translator_id.clone());
        }

        PatchField::Skip => {}
    }

    match revision {
        //
        PatchField::Clear => {
            //
            unit_info.is_proofread = false;

            unit_info.proofread_text = None;

            unit_info.last_proofreader_id = None;
        }

        PatchField::Assign(revision) => {
            //
            unit_info.is_proofread = revision.is_proofread;

            unit_info.proofread_text = revision.proofread_text.clone();

            unit_info.last_proofreader_id =
                Some(revision.last_proofreader_id.clone());
        }

        PatchField::Skip => {}
    }

    unit_info.updated_at = now();
}

fn find_unit_mut<'a>(
    state: &'a mut MockState,
    page_id: &str,
    id: &str,
) -> BaseResult<&'a mut UnitInfo> {
    state
        .units
        .iter_mut()
        .find(|unit_info| unit_info.page_id == page_id && unit_info.id == id)
        .ok_or_else(|| expected("error-invalid-unit-oper"))
}

fn count_infos(unit_infos: &[UnitInfo]) -> UnitCounters {
    unit_infos.iter().fold(
        UnitCounters::default(),
        |mut counters, unit_info| {
            //
            counters.total_unit_count += 1;

            if unit_info.is_translated() {
                counters.translated_unit_count += 1;
            }

            if unit_info.is_proofread {
                counters.proofread_unit_count += 1;
            }

            counters
        },
    )
}

fn order_units<T, I, N>(
    units: &mut [T],
    id_of: I,
    next_id_of: N,
) -> BaseResult<()>
where
    I: for<'a> Fn(&'a T) -> &'a str,
    N: for<'a> Fn(&'a T) -> Option<&'a str>,
{
    //
    if units.is_empty() {
        return accept(());
    }

    for index in 0..units.len() {
        if units[index + 1..]
            .iter()
            .any(|unit| id_of(unit) == id_of(&units[index]))
        {
            return Err(unrecoverable("persisted Unit chain is corrupt"));
        }
    }

    let mut head_position = None;

    for candidate in 0..units.len() {
        //
        let has_predecessor = units.iter().any(|unit| {
            next_id_of(unit)
                .is_some_and(|next_id| next_id == id_of(&units[candidate]))
        });

        if has_predecessor {
            continue;
        }

        if head_position.replace(candidate).is_some() {
            return Err(unrecoverable("persisted Unit chain is corrupt"));
        }
    }

    let Some(head_position) = head_position else {
        return Err(unrecoverable("persisted Unit chain is corrupt"));
    };

    units.swap(0, head_position);

    for index in 0..units.len() - 1 {
        //
        let next_position = units[index + 1..].iter().position(|candidate| {
            next_id_of(&units[index])
                .is_some_and(|next_id| next_id == id_of(candidate))
        });

        let Some(next_position) = next_position else {
            return Err(unrecoverable("persisted Unit chain is corrupt"));
        };

        units.swap(index + 1, index + 1 + next_position);
    }

    if units.last().is_some_and(|unit| next_id_of(unit).is_some()) {
        return Err(unrecoverable("persisted Unit chain is corrupt"));
    }

    accept(())
}
