//! Instr DTOs for the comic domain.

//! Data transfer objects for comic use cases — input parameters and
//! presentation-ready values for the comic aggregate.
//!
//! Timestamps are converted to Unix milliseconds for JSON serialisation.
//! Cover URLs are resolved from object-storage keys via [`ImagePool`].
//!
//! [`ImagePool`]: crate::part::image::ImagePool

use serde::Deserialize;

use crate::model::read::spec::comic::{ComicListKind, ComicListSpec};
use crate::result::{BaseError, BaseRest, accept};
use crate::value::chapter::StageMask;
use crate::value::comic::{ComicInclOpt, ComicWithOpt};
use crate::value::image::{ImageExt, ImageHash};
use crate::value::role::RoleMask;
#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

/// Request to reserve a comic cover upload.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ReserveComicCoverInstr {
    //
    /// SHA-256 identity of the exact cover bytes.
    pub image_hash: ImageHash,
    /// Upload size used for validation and PUT signing.
    pub new_byte_len: u64,
    /// Cover file format.
    pub ext: ImageExt,
}

/// Request to confirm one reserved comic cover version.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct MarkComicCoverUploadedInstr {
    /// Version returned in the cover upload slot.
    pub image_version: u32,
}

/// Input parameters for creating a new comic inside a workset.
///
/// The first chapter is created atomically with the comic. Its subtitle
/// can be customised via `first_chapter_subtitle`; when absent, a
/// locale-aware default (e.g. "Ch. 1") is generated.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateComicInstr {
    //
    /// Parent workset identifier.
    pub workset_id: String,

    /// Comic title.
    pub title: String,
    /// Comic author name.
    pub author: String,
    /// Optional description of the comic.
    pub description: Option<String>,

    /// Optional subtitle for the first chapter created alongside the comic.
    pub first_chapter_subtitle: Option<String>,

    /// Roles assigned to the creator on the first chapter in addition to the
    /// mandatory admin role. Every requested role must exist on the creator's
    /// team membership.
    pub preset_assignment_roles: Option<RoleMask>,
}

/// Input parameters for updating a comic's title, author, and description.
///
/// Cover updates are handled by dedicated endpoints.
///
/// [`reserve_cover`]: crate::usecase::comic::reserve_cover
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateComicInfoInstr {
    //
    /// Comic identifier.
    pub id: String,

    /// Updated comic title.
    pub title: String,
    /// Updated author name.
    pub author: String,
    /// Updated description.
    pub description: Option<String>,
}

/// Input parameters for listing comics within a workset.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ListComicInfosInstr {
    //
    /// Parent workset identifier.
    pub workset_id: String,

    /// Optional fuzzy title filter.
    pub fuzzy_title: Option<String>,
    /// Optional stage mask filter.
    pub stages: Option<u32>,

    /// Optional related data to include in results.
    #[serde(default, rename = "incl")]
    pub incl_opt: Vec<ComicInclOpt>,

    /// Optional expansion options for the result set.
    #[serde(default, rename = "with")]
    pub with_opt: Vec<ComicWithOpt>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: u32,
}

impl TryFrom<ListComicInfosInstr> for ComicListSpec {
    // The error type for invalid listing parameters.
    type Error = BaseError;

    // Convert validated query parameters into the domain list spec.
    fn try_from(instr: ListComicInfosInstr) -> BaseRest<Self> {
        //
        let stages =
            instr.stages.map(StageMask::try_filter_from).transpose()?;

        let kind = match stages {
            //
            Some(stage_mask) => ComicListKind::Stages(stage_mask),

            None => ComicListKind::All,
        };

        accept(Self {
            workset_id: instr.workset_id,
            fuzzy_title: instr.fuzzy_title,
            kind,
            incl_opt: instr.incl_opt,
            offset: instr.offset,
            limit: instr.limit,
        })
    }
}
