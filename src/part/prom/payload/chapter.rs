use serde::{Deserialize, Serialize};

/// Deferred check that advances raw provision after all page uploads finish.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckUploadFinish {
    pub chapter_id: String,
}
