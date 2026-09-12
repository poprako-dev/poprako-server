use serde::{Deserialize, Serialize};

/// Deferred chapter task payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChapterPayload {
    //
    /// Advance raw provision after all page uploads finish.
    TryAdvanceRawProvideStage {
        /// Unique identifier of the chapter to verify upload completion for.
        chapter_id: String,
        /// User that scheduled the check.
        actor_user_id: String,
    },
}
