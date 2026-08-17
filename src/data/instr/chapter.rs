//! Instr DTOs for the chapter domain.

//! Data transfer objects for chapter use cases.

use serde::Deserialize;
#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

use crate::value::chapter::{ChapterInclOpt, Stage, StageOper};
use crate::value::role::RoleMask;

/// Input parameters for creating a new chapter.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateChapterInstr {
    /// Identifier of the parent comic to create the chapter in.
    pub comic_id: String,

    /// Optional display subtitle; defaults to a generated value
    /// if omitted (see [`default_subtitle`]).
    ///
    /// [`default_subtitle`]: crate::complex::chapter::default_subtitle
    pub subtitle: Option<String>,

    /// Roles assigned to the creator in addition to the mandatory admin role.
    /// Every requested role must exist on the creator's team membership.
    pub preset_assignment_roles: Option<RoleMask>,
}

/// Input parameters for listing chapters within a comic.
///
/// `incl` embeds related rows into each item; dotted values implicitly pull
/// in their parent segments.
///
/// Example: `/api/v1/comics/{comic_id}/chapters?incl=comic.workset.team&incl=creator&offset=0&limit=20`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ListChapterInfosInstr {
    /// Parent comic whose chapters to list.
    pub comic_id: String,

    /// Related rows to embed. Repeatable. Values: `comic`, `comic.workset`,
    /// `comic.workset.team`, `comic.creator`, `creator`. Dotted values imply
    /// their parent segments.
    #[serde(default, rename = "incl")]
    pub incl_opt: Vec<ChapterInclOpt>,

    /// Pagination offset: number of chapters to skip before beginning the
    /// result set.
    pub offset: u32,
    /// Maximum number of chapters to return.
    pub limit: u32,
}

/// Input parameters for listing immutable workflow records under one chapter.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ListChapterWorkflowRecordInfosInstr {
    /// Chapter whose activity records are listed.
    pub chapter_id: String,
    /// Pagination offset: number of records to skip before beginning the result set.
    pub offset: u32,
    /// Maximum number of records to return.
    pub limit: u32,
}

/// Input parameters for partially updating a chapter's profile.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateChapterInfoInstr {
    /// Chapter identifier to update.
    pub id: String,

    /// New display subtitle; `None` leaves the current value unchanged.
    pub subtitle: Option<String>,
}

/// Input parameters for updating a chapter's workflow stage.
///
/// Encodes a single operation on a specific stage, e.g. "start translating"
/// on the `translate` stage. The use case layer validates that the
/// transition is legal for the current stage phase before applying it.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateChapterStageInstr {
    /// Chapter identifier to update.
    pub id: String,

    /// Workflow stage to operate on.
    pub stage: Stage,
    /// Operation to apply to the target stage (e.g. start, finish, revert).
    pub oper: StageOper,
}
