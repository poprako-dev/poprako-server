//! Complex domain logic for image lifecycle tracking — deletion and integrity-check ID generation for asynchronous image processing.

/// Domain operations for image lifecycle management: generates unique identifiers for scheduled image deletion and integrity check tasks.
pub struct ImageComplex;

impl ImageComplex {
    /// Generates a unique image deletion-task identifier with an `lm-` prefix using UUID v7.
    pub fn gen_delete_id() -> String {
        format!("lm-{}", uuid::Uuid::now_v7())
    }

    /// Generates a unique image integrity-check identifier with an `lm-` prefix using UUID v7.
    pub fn gen_check_id() -> String {
        format!("lm-{}", uuid::Uuid::now_v7())
    }
}
