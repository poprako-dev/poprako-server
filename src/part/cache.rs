//! Process-local leases, save exclusion, and bounded successful receipts.

/// Annotation save coordination and receipts.
pub mod issue_save;
/// Online-user lease capabilities.
pub mod online_user;
/// Independent cache operation descriptors.
pub mod oper;
/// Unit save coordination and replay mappings.
pub mod unit_save;
