//! Cache adapters independent of database repositories.

/// Shared process-local leases, save coordination, and receipts.
pub mod mem_impl;
