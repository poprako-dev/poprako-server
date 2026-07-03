//! Value types for comment aggregates.

use serde::Deserialize;

use utoipa::ToSchema;

/// Incl opts for comment info queries.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CommentInclOpt {
    User,
}
