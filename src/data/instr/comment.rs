//! Instr DTOs for the comment domain.

//! Data transfer objects for comment use cases.

use serde::Deserialize;

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

use crate::model::read::spec::comment::CommentListSpec;
use crate::value::comment::CommentInclOpt;
use crate::value::pagination::PubListLimit;

/// Input parameters for listing comments.
///
/// `incl` embeds related rows into each item.
///
/// Example: `/api/v1/teams/{team_id}/comments?incl=user&offset=0&limit=20`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ListCommentInfosInstr {
    /// Parent team whose comments to list.
    pub team_id: String,

    /// Related rows to embed. Repeatable. Values: `user`.
    #[serde(default)]
    pub incl: Vec<CommentInclOpt>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: PubListLimit,
}

impl From<ListCommentInfosInstr> for CommentListSpec {
    // Map comment listing parameters directly to the repository spec.
    fn from(instr: ListCommentInfosInstr) -> Self {
        Self {
            team_id: instr.team_id,
            incl: instr.incl,
            offset: instr.offset,
            limit: instr.limit,
        }
    }
}

/// Input parameters for creating a comment.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateCommentInstr {
    /// Target team identifier.
    pub team_id: String,
    /// Comment body text.
    pub content: String,
}
