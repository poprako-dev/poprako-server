//! Value types for comment aggregates.

use serde::Deserialize;

/// Include options for comment info queries.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum CommentInclOpt {
    User,
}
