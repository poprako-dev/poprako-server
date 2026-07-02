//! Value types for comment aggregates.

use serde::Deserialize;

/// Incl opts for comment info queries.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum CommentInclOpt {
    User,
}
