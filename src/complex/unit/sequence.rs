//! Pure reconstruction of complete persisted Unit chains.

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use crate::result::{BaseError, BaseRest, accept};

/// Orders a complete persisted Unit chain without changing record content.
///
/// Empty chains are valid. Duplicate IDs, missing successors, competing
/// predecessors, cycles, and disconnected records are unrecoverable corruption.
/// Reconstruction takes expected linear time and linear auxiliary storage.
pub fn order_units<T, I, N>(
    units: &mut [T],
    id_of: I,
    next_id_of: N,
) -> BaseRest<()>
where
    I: for<'a> Fn(&'a T) -> &'a str,
    N: for<'a> Fn(&'a T) -> Option<&'a str>,
{
    //
    if units.is_empty() {
        return accept(());
    }

    let mut index_by_id = HashMap::with_capacity(units.len());

    for (index, unit) in units.iter().enumerate() {
        //
        if index_by_id.insert(id_of(unit), index).is_some() {
            return Err(corrupt_unit_chain_err());
        }
    }

    let mut next_index_by_index = Vec::with_capacity(units.len());

    let mut has_predecessor = vec![false; units.len()];

    for unit in units.iter() {
        //
        let next_index = match next_id_of(unit) {
            //
            Some(next_id) => {
                //
                let Some(next_index) = index_by_id.get(next_id).copied() else {
                    return Err(corrupt_unit_chain_err());
                };

                let Some(has_predecessor) = has_predecessor.get_mut(next_index)
                else {
                    return Err(corrupt_unit_chain_err());
                };

                if std::mem::replace(has_predecessor, true) {
                    return Err(corrupt_unit_chain_err());
                }

                Some(next_index)
            }

            None => None,
        };

        next_index_by_index.push(next_index);
    }

    let mut head_indexes = has_predecessor.iter().enumerate().filter_map(
        |(index, has_predecessor)| (!has_predecessor).then_some(index),
    );

    let Some(head_index) = head_indexes.next() else {
        return Err(corrupt_unit_chain_err());
    };

    if head_indexes.next().is_some() {
        return Err(corrupt_unit_chain_err());
    }

    let mut ordered_indexes = Vec::with_capacity(units.len());

    let mut current_index = Some(head_index);

    while let Some(index) = current_index {
        //
        if ordered_indexes.len() >= units.len() {
            return Err(corrupt_unit_chain_err());
        }

        ordered_indexes.push(index);

        let Some(next_index) = next_index_by_index.get(index).copied() else {
            return Err(corrupt_unit_chain_err());
        };

        current_index = next_index;
    }

    if ordered_indexes.len() != units.len() {
        return Err(corrupt_unit_chain_err());
    }

    drop(index_by_id);

    let mut original_index_by_position = (0..units.len()).collect::<Vec<_>>();

    let mut position_by_original_index = (0..units.len()).collect::<Vec<_>>();

    for (target_position, desired_original_index) in
        ordered_indexes.into_iter().enumerate()
    {
        //
        let Some(current_position) = position_by_original_index
            .get(desired_original_index)
            .copied()
        else {
            return Err(corrupt_unit_chain_err());
        };

        let Some(displaced_original_index) =
            original_index_by_position.get(target_position).copied()
        else {
            return Err(corrupt_unit_chain_err());
        };

        if target_position >= units.len() || current_position >= units.len() {
            return Err(corrupt_unit_chain_err());
        }

        units.swap(target_position, current_position);

        original_index_by_position.swap(target_position, current_position);

        let Some(desired_position) =
            position_by_original_index.get_mut(desired_original_index)
        else {
            return Err(corrupt_unit_chain_err());
        };

        *desired_position = target_position;

        let Some(displaced_position) =
            position_by_original_index.get_mut(displaced_original_index)
        else {
            return Err(corrupt_unit_chain_err());
        };

        *displaced_position = current_position;
    }

    accept(())
}

// Returns an unrecoverable error for a corrupt Unit chain.
fn corrupt_unit_chain_err() -> BaseError {
    //
    BaseError::Unrecoverable {
        message: "persisted Unit chain is corrupt".to_string(),
    }
}
