//! Value types for comment aggregates.

use serde::Deserialize;

use utoipa::ToSchema;

/// Incl opts for comment info queries.
///
/// Each opt embeds additional related data into the returned
/// `CommentInfoVal`.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CommentInclOpt {
    /// Embed the comment's author (`user`).
    User,
}
