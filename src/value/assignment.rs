//! Value types for assignment aggregates.

use serde::Deserialize;

/// Include options for assignment info queries.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentInclOpt {
    User,
}
