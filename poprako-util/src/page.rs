//! Pagination parameters for offset/limit-based queries.

/// Offset and limit parameters for paginated database queries.
#[derive(Debug, Clone, Copy)]
pub struct Page {
    pub offset: u64,
    pub limit: u64,
}
