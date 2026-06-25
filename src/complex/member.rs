//! Complex domain logic for [Member] aggregates — ID generation for team membership records.

use uuid::Uuid;

/// Domain operations for [Member] aggregates: unique identifier generation.
pub struct MemberComplex;

impl MemberComplex {
    /// Generates a unique member identifier with a `member-` prefix using UUID v7.
    pub fn gen_id() -> String {
        format!("member-{}", Uuid::now_v7())
    }
}
