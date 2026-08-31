//! Val DTOs for the comic domain.

//! Data transfer objects for comic use cases — input parameters and
//! presentation-ready values for the comic aggregate.
//!
//! Timestamps are converted to Unix milliseconds for JSON serialisation.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::data::view::image::ImageUploadSlotView;

/// Comic cover upload allocation response.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AllocComicCoverVal {
    /// Upload capability, absent when this cover is already uploaded.
    pub slot: Option<ImageUploadSlotView>,
}

/// Return value from a successful comic creation.
///
/// Includes the IDs of both the new comic and its auto-created first chapter.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateComicVal {
    //
    /// Newly created comic identifier.
    pub id: String,
    /// Identifier of the auto-created first chapter.
    pub chapter_id: String,
}
