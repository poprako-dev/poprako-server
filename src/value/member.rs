//! Value types for member aggregates.

use serde::Deserialize;
#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::value::incl::InclOpt;

/// Incl opts for member info queries.
///
/// Each opt embeds additional related data into the returned
/// `MemberInfoView`.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum MemberInclOpt {
    //
    /// Embed the member's user profile (`user`).
    User,

    /// Embed the member's team (`team`).
    Team,
}

impl InclOpt for MemberInclOpt {
    // Expand each include request into its dependency-ordered chain.
    fn path(self) -> &'static [Self] {
        //
        match self {
            //
            Self::User => &[Self::User],

            Self::Team => &[Self::Team],
        }
    }
}
