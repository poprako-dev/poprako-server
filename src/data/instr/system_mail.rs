//! Instr DTOs for the system mail domain.

//! Data transfer objects for system mail use cases.

use serde::Deserialize;

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

/// Input parameters for listing system mails.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ListSystemMailInfosInstr {
    /// Filter by read status. Absent returns all.
    pub is_read: Option<bool>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: u32,
}

/// Input parameters for marking a batch of system mails as read.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct MarkSystemMailReadInstr {
    /// Identifiers of the system mails to mark as read.
    pub ids: Vec<String>,
}
