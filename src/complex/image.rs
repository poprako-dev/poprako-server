//! Complex domain logic for image lifecycle tracking — deletion and integrity-check ID generation for asynchronous image processing.

use crate::util::next_snowflake_id;

/// Domain operations for image lifecycle management: generates unique identifiers for scheduled image deletion and integrity check tasks.
pub struct ImageComplex;

impl ImageComplex {
    /// Generates a unique image deletion-task identifier backed by a snowflake value.
    pub fn gen_delete_id() -> String {
        next_snowflake_id()
    }

    /// Generates a unique image integrity-check identifier backed by a snowflake value.
    pub fn gen_check_id() -> String {
        next_snowflake_id()
    }
}
