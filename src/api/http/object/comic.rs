//! HTTP wire objects for the migrated data DTOs.
use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

use crate::api::http::object::team::TeamInfoObj;
use crate::api::http::object::upload::ImageUploadSlotObj;
use crate::api::http::object::user::UserInfoObj;
use crate::api::http::object::workset::WorksetInfoObj;
use crate::api::http::result::Wire;
use crate::data::instr::comic::{
    AllocComicCoverInstr, CreateComicInstr, GetComicInfoInstr,
    ListComicInfosInstr, MarkComicCoverUploadedInstr, UpdateComicInfoInstr,
};
use crate::data::val::comic::{AllocComicCoverVal, CreateComicVal};
use crate::data::view::comic::ComicInfoView;
use crate::data::view::obj_url::ObjUrlView;
use crate::value::comic::{ComicInclOpt, ComicStatus, ComicWithOpt};
use crate::value::image::{ImageExt, ImageHash};
use crate::value::pagination::PubListLimit;
use crate::value::role::RoleMask;

/// Request to allocate a comic cover upload.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AllocComicCoverBody {
    /// SHA-256 identity of the exact cover bytes.
    pub image_hash: ImageHash,
    /// Upload size used for validation and PUT signing.
    pub new_byte_len: u64,
    /// Cover file format.
    pub ext: ImageExt,
}

impl From<AllocComicCoverBody> for AllocComicCoverInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: AllocComicCoverBody) -> Self {
        Self {
            image_hash: value.image_hash,
            new_byte_len: value.new_byte_len,
            ext: value.ext,
        }
    }
}

/// Request to confirm one allocated comic cover version.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct MarkComicCoverUploadedBody {
    /// Version returned in the cover upload slot.
    #[serde(rename = "image_version")]
    pub image_ver: u32,
}

impl From<MarkComicCoverUploadedBody> for MarkComicCoverUploadedInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: MarkComicCoverUploadedBody) -> Self {
        Self {
            image_ver: value.image_ver,
        }
    }
}

/// Input parameters for creating a new comic inside a workset.
///
/// The first chapter is created atomically with the comic. Its subtitle
/// can be customised via `first_chapter_subtitle`; when absent, a
/// locale-aware default (e.g. "Ch. 1") is generated.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateComicBody {
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

impl From<CreateComicBody> for CreateComicInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: CreateComicBody) -> Self {
        Self {
            workset_id: value.workset_id,
            title: value.title,
            author: value.author,
            description: value.description,
            first_chapter_subtitle: value.first_chapter_subtitle,
            preset_assignment_roles: value.preset_assignment_roles,
        }
    }
}

/// Input parameters for updating a comic's title, author, and description.
///
/// Cover updates are handled by dedicated endpoints.
///
/// [`alloc_cover`]: crate::usecase::comic::alloc_cover
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateComicInfoBody {
    /// Comic identifier.
    pub id: String,

    /// Updated comic title.
    pub title: String,
    /// Updated author name.
    pub author: String,
    /// Updated description.
    pub description: Option<String>,
}

impl From<UpdateComicInfoBody> for UpdateComicInfoInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: UpdateComicInfoBody) -> Self {
        Self {
            id: value.id,
            title: value.title,
            author: value.author,
            description: value.description,
        }
    }
}

/// Related data requested when retrieving one comic.
#[derive(Debug, Default, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct GetComicInfoQuery {
    /// Related rows to embed. Dotted values include their parent segments.
    #[serde(default)]
    pub incl: Vec<ComicInclOpt>,
}

impl From<GetComicInfoQuery> for GetComicInfoInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: GetComicInfoQuery) -> Self {
        Self { incl: value.incl }
    }
}

/// Input parameters for listing comics within a workset.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ListComicInfosQuery {
    /// Parent workset identifier.
    pub workset_id: String,

    /// Optional fuzzy title filter.
    pub fuzzy_title: Option<String>,
    /// Optional stage mask filter.
    pub stages: Option<u32>,
    /// Optional lifecycle state filter.
    pub status: Option<ComicStatus>,

    /// Optional related data to include in results.
    #[serde(default)]
    pub incl: Vec<ComicInclOpt>,

    /// Optional expansion options for the result set.
    #[serde(default)]
    pub r#with: Vec<ComicWithOpt>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: PubListLimit,
}

impl From<ListComicInfosQuery> for ListComicInfosInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ListComicInfosQuery) -> Self {
        Self {
            workset_id: value.workset_id,
            fuzzy_title: value.fuzzy_title,
            stages: value.stages,
            status: value.status,
            incl: value.incl,
            r#with: value.r#with,
            offset: value.offset,
            limit: value.limit,
        }
    }
}

/// Comic cover upload allocation response.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AllocComicCoverPayload {
    /// Upload capability, absent when this cover is already uploaded.
    pub slot: Option<ImageUploadSlotObj>,
}

impl From<AllocComicCoverVal> for AllocComicCoverPayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: AllocComicCoverVal) -> Self {
        Self {
            slot: value.slot.map(std::convert::Into::into),
        }
    }
}

impl Wire for AllocComicCoverVal {
    // Serializable wire payload produced by this conversion.
    type Target = AllocComicCoverPayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Return value from a successful comic creation.
///
/// Includes the IDs of both the new comic and its auto-created first chapter.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateComicPayload {
    /// Newly created comic identifier.
    pub id: String,
    /// Identifier of the auto-created first chapter.
    pub chapter_id: String,
}

impl From<CreateComicVal> for CreateComicPayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: CreateComicVal) -> Self {
        Self {
            id: value.id,
            chapter_id: value.chapter_id,
        }
    }
}

impl Wire for CreateComicVal {
    // Serializable wire payload produced by this conversion.
    type Target = CreateComicPayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Presentation-ready comic information.
///
/// Mirrors [`ComicInfo`], converts timestamps to Unix milliseconds, and
/// accepts object URLs already resolved by the use-case layer.
///
/// [`ComicInfo`]: crate::model::read::proj::comic::ComicInfo
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ComicInfoObj {
    /// Unique comic identifier.
    pub id: String,

    /// Parent workset identifier this comic belongs to.
    pub workset_id: String,

    /// Ordinal position of the comic within its workset.
    pub index: usize,

    /// Comic title.
    pub title: String,
    /// Comic author name.
    pub author: String,
    /// Optional description or synopsis of the comic content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Resolved signed download URL for the cover image, or [`None`] if
    /// no cover has been uploaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "swagger", schema(value_type = Option<String>))]
    pub cover_url: Option<String>,
    /// Resolved signed download URL for the cover thumbnail, when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "swagger", schema(value_type = Option<String>))]
    pub cover_thumbnail_url: Option<String>,

    /// Total number of chapters in this comic.
    pub chapter_count: usize,

    /// Identifier of the user who created the comic entry.
    pub creator_id: String,

    /// Resolved workset summary, included when the `workset` expansion option is requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workset: Option<WorksetInfoObj>,
    /// Resolved team summary for the owning workset, included when the `team` expansion option is requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team: Option<TeamInfoObj>,
    /// Resolved creator profile, included when the `creator` expansion option is requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator: Option<UserInfoObj>,

    /// Timestamp of the most recent activity on the comic, in milliseconds since Unix epoch.
    pub last_active_at: i64,

    /// Whether this comic has been archived and is no longer writable.
    pub is_archived: bool,
    /// Timestamp when the comic was archived, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived_at: Option<i64>,

    /// Timestamp of comic creation, in milliseconds since Unix epoch.
    pub created_at: i64,
    /// Timestamp of the last comic update, in milliseconds since Unix epoch.
    pub updated_at: i64,
}

impl From<ComicInfoView> for ComicInfoObj {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ComicInfoView) -> Self {
        Self {
            id: value.id,
            workset_id: value.workset_id,
            index: value.index,
            title: value.title,
            author: value.author,
            description: value.description,
            cover_url: value.cover_url.map(ObjUrlView::into_string),
            cover_thumbnail_url: value
                .cover_thumbnail_url
                .map(ObjUrlView::into_string),
            chapter_count: value.chapter_count,
            creator_id: value.creator_id,
            workset: value.workset.map(std::convert::Into::into),
            team: value.team.map(std::convert::Into::into),
            creator: value.creator.map(std::convert::Into::into),
            last_active_at: value.last_active_at,
            is_archived: value.is_archived,
            archived_at: value.archived_at,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl Wire for ComicInfoView {
    // Serializable wire payload produced by this conversion.
    type Target = ComicInfoObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Query for listing comics within a workset.
///
/// When present, `stages` narrows the list by pinned chapter workflow state.
///
/// `incl` embeds related rows into each item; `with` populates derived rows
/// in the parallel list payload.
/// Dotted `incl` values implicitly pull in their parent segments.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ComicListQuery {
    /// Fuzzy title substring filter (case-insensitive).
    pub fuzzy_title: Option<String>,

    /// Workflow stage bitmask filter for pinned chapters.
    pub stages: Option<u32>,

    /// Lifecycle state filter. Omit to list both active and archived comics.
    pub status: Option<ComicStatus>,

    /// Related rows to embed. Repeatable. Values: `workset`, `workset.team`,
    /// `creator`. Dotted values imply their parent segments.
    #[serde(default)]
    pub incl: Vec<ComicInclOpt>,

    /// Derived rows to attach. Repeatable. Values: `pinned_chapter`,
    /// `pinned_chapter_assignment`. The assignment option requires the chapter
    /// option.
    #[serde(default)]
    pub r#with: Vec<ComicWithOpt>,

    /// Pagination offset (0-based).
    pub offset: u32,

    /// Maximum number of items to return.
    pub limit: PubListLimit,
}
