use serde::{Deserialize, Serialize};

/// Deferred check that advances raw provision after all page uploads finish.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdvanceRawProvide {
    /// Unique identifier of the chapter to verify upload completion for.
    pub chapter_id: String,
}
