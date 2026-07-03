//! Shared conversion helpers for stored zero-based indexes and user-facing indexes.

/// Convert a stored zero-based index into a user-facing one-based index.
pub fn stored_index_to_user_index(index: i32) -> i32 {
    index + 1
}

/// Convert a user-facing one-based index into a stored zero-based index.
pub fn user_index_to_stored_index(index: i32) -> Option<i32> {
    match index {
        1.. => Some(index - 1),
        _ => None,
    }
}
