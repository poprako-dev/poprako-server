//! Instr DTOs for the team domain.

//! Data transfer objects for team profile use cases.

use serde::Deserialize;
#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

use crate::value::image::{ImageExt, ImageHash};

/// Request to reserve a team avatar upload.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ReserveTeamAvatarInstr {
    //
    /// SHA-256 identity of the exact avatar bytes.
    pub image_hash: ImageHash,
    /// Upload size used for validation and PUT signing.
    pub new_byte_len: u64,
    /// Avatar file format.
    pub ext: ImageExt,
}

/// Request to confirm one reserved team avatar version.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct MarkTeamAvatarUploadedInstr {
    /// Version returned in the avatar upload slot.
    pub image_version: u32,
}

/// Input parameters for creating a new team.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateTeamInstr {
    //
    /// Team display name.
    pub name: String,
    /// Team description text.
    pub description: String,
}

/// Input parameters for listing teams.
///
/// Exactly one listing mode applies based on `user_id`:
/// - `user_id` omitted: list all teams (requires super-admin, otherwise
///   `403`);
/// - `user_id` present: list teams the given user has joined.
///
/// Example: `/api/v1/teams?user_id=u_123&offset=0&limit=20`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ListTeamInfosInstr {
    //
    /// Filter to teams joined by this user. Omit to list all teams
    /// (super-admin only).
    pub user_id: Option<String>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: u32,
}

/// Input parameters for updating a team's profile.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateTeamInfoInstr {
    //
    /// Team identifier.
    pub id: String,

    /// Updated team display name.
    pub name: String,
    /// Updated team description text.
    pub description: String,
}
