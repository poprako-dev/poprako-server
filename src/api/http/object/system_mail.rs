//! HTTP wire objects for the migrated data DTOs.
use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

use crate::api::http::result::Wire;
use crate::data::instr::system_mail::ListSystemMailInfosInstr;
use crate::data::view::system_mail::SystemMailInfoView;
use crate::value::pagination::PubListLimit;

/// Input parameters for listing system mails.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ListSystemMailInfosQuery {
    /// Filter by read status. Absent returns all.
    pub is_read: Option<bool>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: PubListLimit,
}

impl From<ListSystemMailInfosQuery> for ListSystemMailInfosInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ListSystemMailInfosQuery) -> Self {
        Self {
            is_read: value.is_read,
            offset: value.offset,
            limit: value.limit,
        }
    }
}

/// Input parameters for marking a batch of system mails as read.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct MarkSystemMailReadBody {
    /// Identifiers of the system mails to mark as read.
    pub ids: Vec<String>,
}

/// Presentation-ready system mail value.
///
/// Converts the raw [`SystemMailInfo`] timestamp to Unix milliseconds
/// and omits the internal `receiver_id` field.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct SystemMailInfoObj {
    /// Unique identifier.
    pub id: String,

    /// Mail title.
    pub title: String,
    /// Mail body content.
    pub content: String,

    /// Whether the mail has been read.
    pub is_read: bool,

    /// Timestamp of creation, in Unix milliseconds.
    pub created_at: i64,
}

impl From<SystemMailInfoView> for SystemMailInfoObj {
    // Converts the value between the use-case and wire shapes.
    fn from(value: SystemMailInfoView) -> Self {
        Self {
            id: value.id,
            title: value.title,
            content: value.content,
            is_read: value.is_read,
            created_at: value.created_at,
        }
    }
}

impl Wire for SystemMailInfoView {
    // Serializable wire payload produced by this conversion.
    type Target = SystemMailInfoObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}
