//! In-memory implementation of Unit repository operations.

// Internal organization of the `orchestra` module.
mod orchestra;

#[cfg(test)]
// Internal organization of the `tests` module.
mod tests;

use std::collections::HashSet;

use crate::model::read::proj::unit::{UnitCountMetrics, UnitInfo, UnitOrder};
use crate::model::write::unit::UnitEdit;
use crate::part_impl::repo::mock_impl::{
    MockState, expected, now, unrecoverable,
};
use crate::result::{BaseRest, accept};
use crate::util::Patch;
use crate::value::unit::MAX_PAGE_UNIT_COUNT;

// Locate a unit id inside ordered id list, used by move/create/save operations.
fn find_order_pos(ordered_ids: &[&str], id: &str) -> Option<usize> {
    //
    ordered_ids
        .iter()
        .enumerate()
        .find_map(|(pos, cand)| (*cand == id).then_some(pos))
}

// Reorders linked-list-like unit slices into a deterministic traversal order.
fn order_units<T, I, N>(
    units: &mut [T],
    id_of: I,
    next_id_of: N,
) -> BaseRest<()>
where
    I: for<'a> Fn(&'a T) -> &'a str,
    N: for<'a> Fn(&'a T) -> Option<&'a str>,
{
    // Validate and reorder to follow next pointers from head to tail.
    if units.is_empty() {
        return accept(());
    }

    for index in 0..units.len() {
        //
        if units[index + 1..]
            .iter()
            .any(|unit| id_of(unit) == id_of(&units[index]))
        {
            return Err(unrecoverable("persisted Unit chain is corrupt"));
        }
    }

    let mut head_pos = None;

    for cand in 0..units.len() {
        //
        // Detect whether this unit has any predecessor.
        let has_predecessor = units.iter().any(|unit| {
            //
            next_id_of(unit)
                .is_some_and(|next_id| next_id == id_of(&units[cand]))
        });

        if has_predecessor {
            continue;
        }

        if head_pos.replace(cand).is_some() {
            return Err(unrecoverable("persisted Unit chain is corrupt"));
        }
    }

    let Some(head_pos) = head_pos else {
        return Err(unrecoverable("persisted Unit chain is corrupt"));
    };

    units.swap(0, head_pos);

    for index in 0..units.len() - 1 {
        //
        // Find the explicit successor and move it directly after current unit.
        let next_pos =
            units[index + 1..]
                .iter()
                .enumerate()
                .find_map(|(pos, cand)| {
                    //
                    next_id_of(&units[index])
                        .is_some_and(|next_id| next_id == id_of(cand))
                        .then_some(pos)
                });

        let Some(next_pos) = next_pos else {
            return Err(unrecoverable("persisted Unit chain is corrupt"));
        };

        units.swap(index + 1, index + 1 + next_pos);
    }

    if units.last().is_some_and(|unit| next_id_of(unit).is_some()) {
        return Err(unrecoverable("persisted Unit chain is corrupt"));
    }

    accept(())
}

// Move one unit id to a new position based on requested next_id.
fn move_order<'a>(
    ordered_ids: &mut Vec<&'a str>,
    id: &'a str,
    next_id: Option<&str>,
) -> BaseRest<()> {
    //
    // Return an operation error if caller targets an unknown id.
    let Some(pos) = find_order_pos(ordered_ids, id) else {
        return Err(expected("error-invalid-unit-oper"));
    };

    let id = ordered_ids.remove(pos);

    let pos = match next_id {
        //
        // Insert at explicit next-id position.
        Some(next_id) => find_order_pos(ordered_ids, next_id)
            .ok_or_else(|| expected("error-invalid-unit-oper"))?,

        // Append to tail when next-id is omitted.
        None => ordered_ids.len(),
    };

    ordered_ids.insert(pos, id);

    accept(())
}

// List units for one page in deterministic next-id order.
fn list_infos(state: &MockState, page_id: &str) -> BaseRest<Vec<UnitInfo>> {
    //
    // Load all units and enforce chain ordering before returning.
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

    accept(unit_infos)
}

// Apply edit instructions into an ordered id list and return new unit traversal order.
fn apply_order_edits<'a>(
    orders: &'a [UnitOrder],
    edits: &'a [UnitEdit],
) -> BaseRest<Vec<&'a str>> {
    //
    // Start with persisted ordering, then apply create/save/delete repositioning rules.
    let mut ordered_ids = orders
        .iter()
        .map(|order| order.id.as_str())
        .collect::<Vec<_>>();

    let mut hidden_ids = orders
        .iter()
        .filter(|order| order.is_hidden)
        .map(|order| order.id.as_str())
        .collect::<HashSet<_>>();

    for edit in edits {
        //
        match edit {
            //
            // Create may append a brand-new id and optionally re-anchor.
            UnitEdit::Create { id, next_id, .. } => {
                //
                if find_order_pos(&ordered_ids, id).is_some() {
                    return Err(expected("error-invalid-unit-oper"));
                }

                ordered_ids.push(id);

                hidden_ids.remove(id.as_str());

                move_order(&mut ordered_ids, id, next_id.as_deref())?;
            }

            // Save can keep visibility and optionally change successor.
            UnitEdit::Save { id, next_id, .. } => {
                //
                hidden_ids.remove(id.as_str());

                match next_id {
                    //
                    Patch::Skip => {}

                    Patch::Clear => {
                        move_order(&mut ordered_ids, id, None)?;
                    }

                    Patch::Assign { value: next_id } => {
                        move_order(&mut ordered_ids, id, Some(next_id))?;
                    }
                }
            }

            // Delete marks unit ids as hidden for count/publish checks.
            UnitEdit::Delete { id } => {
                hidden_ids.insert(id);
            }
        }
    }

    let visible_count = ordered_ids
        .iter()
        .filter(|id| !hidden_ids.contains(**id))
        .count();

    if visible_count > MAX_PAGE_UNIT_COUNT {
        return Err(expected("error-invalid-unit-oper"));
    }

    accept(ordered_ids)
}

// Build a unit payload for create edits.
fn unit_from_edit(page_id: &str, edit: &UnitEdit) -> BaseRest<UnitInfo> {
    //
    // Parse creation fields and initialize runtime state with now timestamps.
    let UnitEdit::Create {
        id,
        is_bubble,
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
fn count_infos(unit_infos: &[UnitInfo]) -> UnitCountMetrics {
    //
    // Produce summary fields for response after edits are applied.
    unit_infos
        .iter()
        .filter(|unit_info| unit_info.hidden_at.is_none())
        .fold(UnitCountMetrics::default(), |mut counters, unit_info| {
            //
            counters.total += 1;

            if unit_info.is_translated() {
                counters.translated += 1;
            }

            if unit_info.is_proofread {
                counters.proofread += 1;
            }

            counters
        })
}

// List units for multiple pages while preserving page and linked-list order.
fn list_infos_by_page_ids(
    state: &MockState,
    page_ids: &[String],
) -> BaseRest<Vec<UnitInfo>> {
    //
    let mut unit_infos = Vec::new();

    for page_id in page_ids {
        unit_infos.extend(list_infos(state, page_id)?);
    }

    accept(unit_infos)
}

// List every requested persisted Unit that currently exists.
fn list_infos_by_ids(state: &MockState, ids: &[String]) -> Vec<UnitInfo> {
    //
    state
        .units
        .iter()
        .filter(|unit_info| ids.contains(&unit_info.id))
        .cloned()
        .collect()
}

// List lightweight order objects for one page, kept aligned with unit sequence.
fn list_orders(state: &MockState, page_id: &str) -> BaseRest<Vec<UnitOrder>> {
    //
    // Load order metadata and then sort by linked-list order.
    let mut orders = state
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
        &mut orders,
        |unit_info| unit_info.id.as_str(),
        |unit_info| unit_info.next_id.as_deref(),
    )?;

    accept(orders)
}

// Apply all create/save/delete edits for one page and return unit counters.
fn apply_edits(
    state: &mut MockState,
    page_id: &str,
    orders: &[UnitOrder],
    edits: &[UnitEdit],
) -> BaseRest<UnitCountMetrics> {
    //
    // Derive a new order first, then apply each edit with conflict checks.
    let ordered_ids = apply_order_edits(orders, edits)?;

    for edit in edits {
        //
        match edit {
            //
            // Insert new unit or change existing unit state.
            UnitEdit::Create { id, .. } => {
                //
                let unit_info = unit_from_edit(page_id, edit)?;

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

    for (index, id) in ordered_ids.iter().enumerate() {
        //
        // Write normalized next-id links after all edits are applied.
        let next_id = ordered_ids.get(index + 1);

        let unit_info = find_unit_mut(state, page_id, id)?;

        unit_info.next_id = next_id.map(|next_id| (*next_id).to_string());

        unit_info.updated_at = now();
    }

    let unit_infos = list_infos(state, page_id)?;

    accept(count_infos(&unit_infos))
}
