//! Domain models for comics inside worksets — core metadata, cover-storage
//! tracking, and denormalised chapter counters.
//!
//! Convert to [`ComicInfoVal`] for presentation outside the domain layer.
//!
//! [`ComicInfoVal`]: crate::data::val::comic::ComicInfoVal

use time::OffsetDateTime;

use crate::model::read::proj::team::TeamInfo;
use crate::model::read::proj::user::UserInfo;
use crate::model::read::proj::workset::WorksetInfo;
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
