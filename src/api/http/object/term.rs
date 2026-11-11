//! HTTP wire objects for the migrated data DTOs.
use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

use crate::api::http::result::Wire;
use crate::data::instr::term::{
    CreateTermInstr, ListTermInfosInstr, UpdateTermInfoInstr,
};
use crate::data::val::term::CreateTermVal;
use crate::data::view::term::TermInfoView;
use crate::value::pagination::PubListLimit;

/// Input parameters for creating a terminology entry.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateTermBody {
    /// Parent terminology base identifier.
    pub termbase_id: String,

    /// Source-language term text.
    pub source: String,
    /// Target-language translations.
    pub targets: Vec<String>,
    /// Optional annotation; absent when no comment is provided.
    pub comment: Option<String>,
}

impl From<CreateTermBody> for CreateTermInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: CreateTermBody) -> Self {
        Self {
            termbase_id: value.termbase_id,
            source: value.source,
            targets: value.targets,
            comment: value.comment,
        }
    }
}

/// Input parameters for replacing terminology-entry fields.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateTermInfoBody {
    /// Identifier of the terminology entry to update.
    pub id: String,

    /// Updated source-language term text.
    pub source: String,
    /// Updated target-language translations.
    pub targets: Vec<String>,
    /// Optional updated annotation; absent when no comment is provided.
    pub comment: Option<String>,
}

impl From<UpdateTermInfoBody> for UpdateTermInfoInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: UpdateTermInfoBody) -> Self {
        Self {
            id: value.id,
            source: value.source,
            targets: value.targets,
            comment: value.comment,
        }
    }
}

/// Input parameters for listing terms inside one terminology base.
#[derive(Debug)]
pub struct ListTermInfosBody {
    /// Parent terminology base identifier.
    pub termbase_id: String,

    /// Optional fuzzy-search pattern for source-language terms; absent when no term filter is applied.
    pub fuzzy_source: Option<String>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: PubListLimit,
}

impl From<ListTermInfosBody> for ListTermInfosInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ListTermInfosBody) -> Self {
        Self {
            termbase_id: value.termbase_id,
            fuzzy_source: value.fuzzy_source,
            offset: value.offset,
            limit: value.limit,
        }
    }
}

/// Return value from creating a terminology entry.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateTermPayload {
    /// Identifier of the newly created terminology entry.
    pub id: String,
}

impl From<CreateTermVal> for CreateTermPayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: CreateTermVal) -> Self {
        Self { id: value.id }
    }
}

impl Wire for CreateTermVal {
    // Serializable wire payload produced by this conversion.
    type Target = CreateTermPayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Presentation-ready terminology-entry information.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct TermInfoObj {
    /// Unique identifier of the terminology entry.
    pub id: String,

    /// Parent terminology base identifier.
    pub termbase_id: String,

    /// Source-language term text.
    pub source: String,
    /// Target-language translations.
    pub targets: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional annotation; absent when the entry has no comment.
    pub comment: Option<String>,

    /// Identifier of the user who created this entry.
    pub creator_id: String,

    /// Unix timestamp in milliseconds of creation.
    pub created_at: i64,
    /// Unix timestamp in milliseconds of the last update.
    pub updated_at: i64,
}

impl From<TermInfoView> for TermInfoObj {
    // Converts the value between the use-case and wire shapes.
    fn from(value: TermInfoView) -> Self {
        Self {
            id: value.id,
            termbase_id: value.termbase_id,
            source: value.source,
            targets: value.targets,
            comment: value.comment,
            creator_id: value.creator_id,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl Wire for TermInfoView {
    // Serializable wire payload produced by this conversion.
    type Target = TermInfoObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Query parameters for terms inside one terminology base.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct TermListQuery {
    /// Optional case-insensitive source substring.
    pub fuzzy_source: Option<String>,

    /// Number of records to skip for pagination.
    pub offset: u32,
    /// Maximum number of records to return.
    pub limit: PubListLimit,
}
