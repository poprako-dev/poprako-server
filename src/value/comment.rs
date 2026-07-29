//! Value types for comment aggregates.

use serde::Deserialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::value::incl::InclOpt;

/// Incl opts for comment info queries.
///
/// Each opt embeds additional related data into the returned
/// `CommentInfoVal`.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum CommentInclOpt {
    /// Embed the comment's author (`user`).
    User,
}

impl InclOpt for CommentInclOpt {
    // Expand include option to concrete dependency expansion.
    fn path(self) -> &'static [Self] {
        match self {
            Self::User => &[Self::User],
        }
    }
}
