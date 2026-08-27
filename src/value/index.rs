//! Shared conversion helpers for stored zero-based indexes and user-facing indexes.

/// Convert a stored zero-based index into a user-facing one-based index.
pub const fn stored_index_to_user_index(index: usize) -> usize {
    index + 1
}

/// Convert a user-facing one-based index into a stored zero-based index.
pub const fn user_index_to_stored_index(index: usize) -> Option<usize> {
    //
    match index {
        //
        1.. => Some(index - 1),

        _ => None,
    }
}
