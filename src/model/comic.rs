//! Domain models for comics inside worksets — core metadata, cover-storage
//! tracking, and denormalised chapter counters.
//!
//! Convert to [`ComicInfoVal`] for presentation outside the domain layer.
//!
//! [`ComicInfoVal`]: crate::data::comic::ComicInfoVal

use time::OffsetDateTime;

use crate::model::team::TeamInfo;
use crate::model::user::UserInfo;
use crate::model::workset::WorksetInfo;
use crate::value::chapter::StageMask;
use crate::value::comic::ComicInclOpt;
use crate::value::image::{ImageExt, ImageHash};

/// A comicrecord as stored in the database.
///
/// Each comic belongs to exactly one workset. Cover uploads follow a multi-step
/// flow: a key is reserved via
/// [`ReserveComicCover`], the client uploads to that key, then
/// the upload is confirmed via [`MarkComicCoverUploaded`].
///
/// [`ReserveComicCover`]: crate::part::repo::oper::comic::ReserveComicCover
/// [`MarkComicCoverUploaded`]: crate::part::repo::oper::comic::MarkComicCoverUploaded
#[derive(Clone)]
pub struct ComicInfo {
    //
    /// Unique identifier for the comic record.
    pub id: String,

    /// The workset this comic belongs to.
    pub workset_id: String,
    /// Sorting position of the comic within its workset.
    pub index: i32,

    /// Display title of the comic series or volume.
    pub title: String,
    /// Name of the author or artist responsible for this comic.
    pub author: String,
    /// Optional longer synopsis or editorial notes about the comic.
    pub description: Option<String>,

    /// Object-storage key for the comic cover image, if one has been reserved.
    pub cover_key: Option<String>,
    /// Whether the client has confirmed the cover upload to the reserved key.
    pub is_cover_uploaded: bool,
    /// Monotonically increasing version counter for optimistic concurrency on cover updates.
    pub cover_version: u32,
    /// SHA-256 identity of the reserved cover content.
    pub cover_hash: ImageHash,
    /// File format persisted with the cover identity.
    pub cover_ext: ImageExt,

    /// Denormalised count of chapters attached to this comic.
    pub chapter_count: i32,

    /// The user who created this comic record.
    pub creator_id: String,

    /// The resolved workset record, populated when the include option is set.
    pub workset: Option<WorksetInfo>,
    /// The resolved team record for the owning workset, populated when requested.
    pub team: Option<TeamInfo>,
    /// The resolved creator user record, populated when the include option is set.
    pub creator: Option<UserInfo>,

    /// Timestamp of the most recent activity on any chapter under this comic.
    pub last_active_at: OffsetDateTime,

    /// When this comic record was first inserted.
    pub created_at: OffsetDateTime,
    /// When this comic record was last updated.
    pub updated_at: OffsetDateTime,
}

/// The data needed to insert a new comic row.
///
/// Supplied at comic-creation time. The `id` is typically generated via
/// [`ComicComplex::gen_id`]; the `index` is allocated by the repo layer.
///
/// [`ComicComplex::gen_id`]: crate::complex::comic::ComicComplex::gen_id
#[cfg_attr(test, derive(Clone))]
pub struct ComicEntry {
    //
    /// Unique identifier for the new comic.
    pub id: String,

    /// The workset this comic will be created under.
    pub workset_id: String,
    /// Sorting position assigned by the insertion logic.
    pub index: i32,

    /// Display title of the comic series or volume.
    pub title: String,
    /// Name of the author or artist responsible for this comic.
    pub author: String,
    /// Optional synopsis or editorial notes to accompany the comic.
    pub description: Option<String>,

    /// The user who creates this comic record.
    pub creator_id: String,
}

/// Mutable profile (non-cover, non-counter) fields for a comic.
#[cfg_attr(test, derive(Clone))]
pub struct ComicInfoUpdate {
    //
    /// Identifies which comic record to update.
    pub id: String,

    /// Updated display title of the comic series or volume.
    pub title: String,
    /// Updated name of the author or artist.
    pub author: String,
    /// Updated synopsis or editorial notes.
    pub description: Option<String>,
}

/// Filtering and pagination parameters for listing comics within a workset.
pub struct ComicInfoListSpec {
    //
    /// The workset whose comics should be listed.
    pub workset_id: String,

    /// Optional fuzzy title search to narrow the results.
    pub fuzzy_title: Option<String>,
    /// Workflow-stage filter controlling which comics are returned.
    pub kind: ComicInfoListKind,

    /// Additional data to include in each result, such as the workset or creator.
    pub incl_opt: Vec<ComicInclOpt>,

    /// Number of records to skip for pagination.
    pub offset: u32,
    /// Maximum number of records to return.
    pub limit: u32,
}

/// Workflow-stage filtering mode for listing comics.
pub enum ComicInfoListKind {
    /// Include all comics regardless of workflow stage.
    All,

    /// Include only comics whose chapters have any of the specified stages.
    Stages(StageMask),
}

/// The result of reserving a new comic cover upload slot.
///
/// Mirrors [`TeamAvatarReservation`] for the comic domain. Contains the
/// generated object-storage key for the client to PUT to, the previous key
/// (if any) to clean up after the new upload succeeds, and the version
/// number that must match when marking the upload complete.
///
/// [`TeamAvatarReservation`]: crate::model::team::TeamAvatarReservation
#[cfg_attr(test, derive(Clone))]
pub struct ComicCoverReservation {
    //
    /// The newly generated object-storage key the client should upload the cover to.
    pub object_key: String,
    /// The previous cover key that should be cleaned up after a successful upload, if any.
    pub prev_object_key: Option<String>,
    /// Version that must match when the upload is marked as complete.
    pub cover_version: u32,
    /// Whether a PUT capability and delayed check are required.
    pub is_upload_required: bool,
}
