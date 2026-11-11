//! HTTP wire objects for the migrated data DTOs.
use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

use crate::api::http::result::Wire;
use crate::data::instr::termbase::{
    CreateTermbaseInstr, ListComicTermbaseInfosInstr,
    ListTeamTermbaseInfosInstr, UpdateTermbaseInfoInstr,
};
use crate::data::val::termbase::CreateTermbaseVal;
use crate::data::view::termbase::TermbaseInfoView;
use crate::value::pagination::PubListLimit;

/// Input parameters for creating a terminology base.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateTermbaseBody {
    /// Team scope identifier; absent for comic-scoped termbases.
    pub team_id: Option<String>,
    /// Comic scope identifier; absent for team-scoped termbases.
    pub comic_id: Option<String>,

    /// Human-readable name for the new terminology base.
    pub name: String,
    /// Optional longer description.
    pub description: Option<String>,
}

impl From<CreateTermbaseBody> for CreateTermbaseInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: CreateTermbaseBody) -> Self {
        Self {
            team_id: value.team_id,
            comic_id: value.comic_id,
            name: value.name,
            description: value.description,
        }
    }
}

/// Input parameters for replacing terminology-base profile fields.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateTermbaseInfoBody {
    /// Terminology-base identifier to update.
    pub id: String,

    /// Updated human-readable name for the terminology base.
    pub name: String,
    /// Updated description for the terminology base.
    pub description: Option<String>,
}

impl From<UpdateTermbaseInfoBody> for UpdateTermbaseInfoInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: UpdateTermbaseInfoBody) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
        }
    }
}

/// Input parameters for listing team-owned terminology bases.
#[derive(Debug)]
pub struct ListTeamTermbaseInfosBody {
    /// Owning team identifier.
    pub team_id: String,

    /// Optional fuzzy name filter for termbase search.
    pub fuzzy_name: Option<String>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: PubListLimit,
}

impl From<ListTeamTermbaseInfosBody> for ListTeamTermbaseInfosInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ListTeamTermbaseInfosBody) -> Self {
        Self {
            team_id: value.team_id,
            fuzzy_name: value.fuzzy_name,
            offset: value.offset,
            limit: value.limit,
        }
    }
}

/// Input parameters for listing terminology bases visible from a comic.
#[derive(Debug)]
pub struct ListComicTermbaseInfosBody {
    /// Owning comic identifier.
    pub comic_id: String,

    /// Optional fuzzy name filter for termbase search.
    pub fuzzy_name: Option<String>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: PubListLimit,
}

impl From<ListComicTermbaseInfosBody> for ListComicTermbaseInfosInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ListComicTermbaseInfosBody) -> Self {
        Self {
            comic_id: value.comic_id,
            fuzzy_name: value.fuzzy_name,
            offset: value.offset,
            limit: value.limit,
        }
    }
}

/// Return value from creating a terminology base.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateTermbasePayload {
    /// Identifier of the newly created terminology base.
    pub id: String,
}

impl From<CreateTermbaseVal> for CreateTermbasePayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: CreateTermbaseVal) -> Self {
        Self { id: value.id }
    }
}

impl Wire for CreateTermbaseVal {
    // Serializable wire payload produced by this conversion.
    type Target = CreateTermbasePayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Presentation-ready terminology-base information.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct TermbaseInfoObj {
    /// Unique terminology-base identifier.
    pub id: String,

    /// Owning team identifier; absent when the termbase is comic-scoped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team_id: Option<String>,
    /// Owning comic identifier; absent when the termbase is team-scoped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comic_id: Option<String>,

    /// Human-readable terminology-base name.
    pub name: String,
    /// Optional longer description of the terminology base.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Number of terms in the terminology base.
    pub term_count: usize,

    /// Identifier of the user who created the terminology base.
    pub creator_id: String,

    /// Timestamp of creation, in milliseconds since Unix epoch.
    pub created_at: i64,
    /// Timestamp of last update, in milliseconds since Unix epoch.
    pub updated_at: i64,
}

impl From<TermbaseInfoView> for TermbaseInfoObj {
    // Converts the value between the use-case and wire shapes.
    fn from(value: TermbaseInfoView) -> Self {
        Self {
            id: value.id,
            team_id: value.team_id,
            comic_id: value.comic_id,
            name: value.name,
            description: value.description,
            term_count: value.term_count,
            creator_id: value.creator_id,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl Wire for TermbaseInfoView {
    // Serializable wire payload produced by this conversion.
    type Target = TermbaseInfoObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Query parameters for terminology-base lists.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct TermbaseListQuery {
    /// Optional case-insensitive name substring.
    pub fuzzy_name: Option<String>,

    /// Number of records to skip for pagination.
    pub offset: u32,
    /// Maximum number of records to return.
    pub limit: PubListLimit,
}
