//! In-memory implementation of Unit repository operations.

// Internal organization of the `orchestra` module.
mod orchestra;

#[cfg(test)]
// Internal organization of the `tests` module.
mod tests;

use crate::complex::unit as unit_complex;
use crate::complex::unit::sequence as unit_sequence_complex;
use crate::model::read::proj::unit::{UnitCountMetrics, UnitInfo, UnitOrder};
use crate::model::write::unit::UnitEdit;
use crate::part_impl::repo::mock_impl::{MockState, expected, now};
use crate::result::{BaseRest, accept};
use crate::util::Patch;

// List units for one page in deterministic next-id order.
fn list_infos<'a>(
    state: &'a MockState,
    page_id: &str,
) -> BaseRest<Vec<&'a UnitInfo>> {
    //
    // Load all units and enforce chain ordering before returning.
    let mut unit_infos = state
        .units
        .iter()
        .filter(|unit_info| unit_info.page_id == page_id)
        .collect::<Vec<_>>();

    unit_sequence_complex::order_units(
        &mut unit_infos,
        |unit_info| unit_info.id.as_str(),
        |unit_info| unit_info.next_id.as_deref(),
    )?;

    accept(unit_infos)
}

// Build a unit payload for create edits.
fn unit_from_edit(
    page_id: &str,
    edit: &UnitEdit,
    next_id: Option<&str>,
) -> BaseRest<UnitInfo> {
    //
    // Parse creation fields and initialize runtime state with now timestamps.
    let UnitEdit::Create {
        id,
        is_bubble,
        is_flagged,
        coord,
        translation,
        revision,
        ..
    } = edit
    else {
        return Err(expected("error-invalid-unit-oper"));
    };

    let (translated_text, last_translator_id) = match translation {
        //
        Some(translation) => (
            Some(translation.translated_text.clone()),
            Some(translation.last_translator_id.clone()),
        ),

        None => (None, None),
    };

    let (is_proofread, proofread_text, last_proofreader_id) = match revision {
        //
        Some(revision) => (
            revision.is_proofread,
            revision.proofread_text.clone(),
            Some(revision.last_proofreader_id.clone()),
        ),

        None => (false, None, None),
    };

    let current_time = now();

    accept(UnitInfo {
        id: id.clone(),
        page_id: page_id.to_string(),
        next_id: next_id.map(str::to_string),
        is_bubble: *is_bubble,
        is_flagged: *is_flagged,
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

// Resolve a single mutable unit by page and id for edit application.
fn find_unit_mut<'a>(
    state: &'a mut MockState,
    page_id: &str,
    id: &str,
) -> BaseRest<&'a mut UnitInfo> {
    //
    state
        .units
        .iter_mut()
        .find(|unit_info| unit_info.page_id == page_id && unit_info.id == id)
        .ok_or_else(|| expected("error-invalid-unit-oper"))
}

// Apply patch values from save edit onto an existing unit model.
fn write_edit(unit_info: &mut UnitInfo, edit: &UnitEdit) {
    //
    // Update textual and proofread fields and bump timestamp.
    let UnitEdit::Save {
        is_bubble,
        is_flagged,
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

    if let Some(is_flagged) = is_flagged {
        unit_info.is_flagged = *is_flagged;
    }

    if let Some(coord) = coord {
        unit_info.coord = coord.clone();
    }

    match translation {
        //
        Patch::Clear => {
            //
            unit_info.translated_text = None;

            unit_info.last_translator_id = None;
        }

        Patch::Assign { value: translation } => {
            //
            unit_info.translated_text =
                Some(translation.translated_text.clone());

            unit_info.last_translator_id =
                Some(translation.last_translator_id.clone());
        }

        Patch::Skip => {}
    }

    match revision {
        //
        Patch::Clear => {
            //
            unit_info.is_proofread = false;

            unit_info.proofread_text = None;

            unit_info.last_proofreader_id = None;
        }

        Patch::Assign { value: revision } => {
            //
            unit_info.is_proofread = revision.is_proofread;

            unit_info.proofread_text = revision.proofread_text.clone();

            unit_info.last_proofreader_id =
                Some(revision.last_proofreader_id.clone());
        }

        Patch::Skip => {}
    }

    unit_info.updated_at = now();
}

// Count translated/proofread units among visible units only.
fn count_infos(unit_infos: &[&UnitInfo]) -> UnitCountMetrics {
    //
    // Produce summary fields for response after edits are applied.
    unit_infos
        .iter()
        .filter(|unit_info| unit_info.hidden_at.is_none())
        .fold(
            UnitCountMetrics::default(),
            |mut count_metrics, unit_info| {
                //
                count_metrics.total += 1;

                if unit_info.is_translated() {
                    count_metrics.translated += 1;
                }

                if unit_info.is_proofread {
                    count_metrics.proofread += 1;
                }

                count_metrics
            },
        )
}

// List units for multiple pages while preserving page and linked-list order.
fn list_infos_by_page_ids(
    state: &MockState,
    page_ids: &[&str],
) -> BaseRest<Vec<UnitInfo>> {
    //
    let mut unit_infos = Vec::new();

    for page_id in page_ids {
        unit_infos.extend(list_infos(state, page_id)?.into_iter().cloned());
    }

    accept(unit_infos)
}

// List every requested persisted Unit that currently exists.
fn list_infos_by_ids(state: &MockState, ids: &[&str]) -> Vec<UnitInfo> {
    //
    state
        .units
        .iter()
        .filter(|unit_info| ids.contains(&unit_info.id.as_str()))
        .cloned()
        .collect()
}

// List lightweight order objects for one page, kept aligned with unit sequence.
fn list_orders(state: &MockState, page_id: &str) -> BaseRest<Vec<UnitOrder>> {
    //
    let unit_infos = list_infos(state, page_id)?;

    accept(
        unit_infos
            .into_iter()
            .map(|unit_info| UnitOrder {
                id: unit_info.id.clone(),
                next_id: unit_info.next_id.clone(),
                is_hidden: unit_info.hidden_at.is_some(),
            })
            .collect(),
    )
}

// Apply all create/save/delete edits for one page and return unit counters.
fn apply_edits(
    state: &mut MockState,
    page_id: &str,
    orders: &[UnitOrder],
    edits: &[UnitEdit],
) -> BaseRest<UnitCountMetrics> {
    //
    // Derive the final sequence once through the shared pure business planner.
    let sequence_plan = unit_complex::plan_edit_sequence(orders, edits)?;

    for edit in edits {
        //
        match edit {
            //
            // Insert new unit or change existing unit state.
            UnitEdit::Create { id, .. } => {
                //
                let next_id = sequence_plan.next_id(id)?;

                let unit_info = unit_from_edit(page_id, edit, next_id)?;

                if state.units.iter().any(|unit_info| unit_info.id == *id) {
                    return Err(expected("error-unit-duplicate"));
                }

                state.units.push(unit_info);
            }

            // Mark visible delete by timestamp but keep ordering slot.
            UnitEdit::Delete { id } => {
                //
                let unit_info = find_unit_mut(state, page_id, id)?;

                unit_info.hidden_at = Some(now());

                unit_info.updated_at = now();
            }

            // Update fields and keep node visible for traversal.
            UnitEdit::Save { id, .. } => {
                //
                let unit_info = find_unit_mut(state, page_id, id)?;

                write_edit(unit_info, edit);
            }
        }
    }

    for successor_change in sequence_plan.changed_successors() {
        //
        let unit_info = find_unit_mut(state, page_id, successor_change.id())?;

        unit_info.next_id = successor_change.next_id().map(str::to_string);

        unit_info.updated_at = now();
    }

    let unit_infos = list_infos(state, page_id)?;

    accept(count_infos(&unit_infos))
}
