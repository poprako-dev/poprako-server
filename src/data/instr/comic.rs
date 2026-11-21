//! Instr DTOs for the comic domain.
//! Data transfer objects for comic use cases — input parameters and
//! presentation-ready values for the comic aggregate.
//!
//! Timestamps are converted to Unix milliseconds for JSON serialisation.
use crate::model::read::spec::comic::ComicListSpec;
use crate::result::{BaseError, BaseRest};
use crate::value::chapter::mask::StageMask;
use crate::value::comic::{ComicInclOpt, ComicStatus, ComicWithOpt};
use crate::value::image::{ImageExt, ImageHash};
use crate::value::pagination::PubListLimit;
use crate::value::role::RoleMask;

/// Request to allocate a comic cover upload.
#[derive(Debug)]
pub struct AllocComicCoverInstr {
    /// SHA-256 identity of the exact cover bytes.
    pub image_hash: ImageHash,
    /// Upload size used for validation and PUT signing.
    pub new_byte_len: u64,
    /// Cover file format.
    pub ext: ImageExt,
}

/// Request to confirm one allocated comic cover version.
#[derive(Debug)]
pub struct MarkComicCoverUploadedInstr {
    /// Version returned in the cover upload slot.
    pub image_ver: u32,
}

/// Input parameters for creating a new comic inside a workset.
///
/// The first chapter is created atomically with the comic. Its subtitle
/// can be customised via `first_chapter_subtitle`; when absent, a
/// locale-aware default (e.g. "Ch. 1") is generated.
#[derive(Debug)]
pub struct CreateComicInstr {
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

    /// Optional worker roles assigned to the creator on the first chapter.
    /// Absent means no assignment. Every requested role must exist on the
    /// creator's team membership.
    pub preset_assignment_roles: Option<RoleMask>,
}

/// Input parameters for updating a comic's title, author, and description.
///
/// Cover updates are handled by dedicated endpoints.
///
/// [`alloc_cover`]: crate::usecase::comic::alloc_cover
#[derive(Debug)]
pub struct UpdateComicInfoInstr {
    /// Comic identifier.
    pub id: String,

    /// Updated comic title.
    pub title: String,
    /// Updated author name.
    pub author: String,
    /// Updated description.
    pub description: Option<String>,
}

/// Related data requested when retrieving one comic.
#[derive(Debug, Default)]
pub struct GetComicInfoInstr {
    /// Related rows to embed. Dotted values include their parent segments.
    pub incl: Vec<ComicInclOpt>,
}

/// Input parameters for listing comics within a workset.
#[derive(Debug)]
pub struct ListComicInfosInstr {
    /// Parent workset identifier.
    pub workset_id: String,

    /// Optional fuzzy title filter.
    pub fuzzy_title: Option<String>,
    /// Optional stage mask filter.
    pub stages: Option<u32>,
    /// Optional lifecycle state filter.
    pub status: Option<ComicStatus>,

    /// Optional related data to include in results.
    pub incl: Vec<ComicInclOpt>,

    /// Optional expansion options for the result set.
    pub r#with: Vec<ComicWithOpt>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: PubListLimit,
}

impl TryFrom<ListComicInfosInstr> for ComicListSpec {
    // The error type for invalid listing parameters.
    type Error = BaseError;

    // Convert validated query parameters into the domain list spec.
    fn try_from(instr: ListComicInfosInstr) -> BaseRest<Self> {
        //
        let stages =
            instr.stages.map(StageMask::try_filter_from).transpose()?;

        accept!(Self {
            workset_id: instr.workset_id,
            fuzzy_title: instr.fuzzy_title,
            stages,
            status: instr.status,
            incl: instr.incl,
            offset: instr.offset,
            limit: instr.limit,
        })
    }
}
