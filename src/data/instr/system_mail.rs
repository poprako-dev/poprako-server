//! Instr DTOs for the system mail domain.
//! Data transfer objects for system mail use cases.
use crate::value::pagination::PubListLimit;

/// Input parameters for listing system mails.
#[derive(Debug)]
pub struct ListSystemMailInfosInstr {
    /// Filter by read status. Absent returns all.
    pub is_read: Option<bool>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: PubListLimit,
}
