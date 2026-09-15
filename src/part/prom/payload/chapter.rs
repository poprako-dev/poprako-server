use serde::{Deserialize, Serialize};

use crate::part::prom::topic::Topic;

/// Deferred chapter task payload.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug))]
pub enum ChapterPayload {
    /// Advance raw provision after all page uploads finish.
    TryAdvanceRawProvideStage {
        /// Unique identifier of the chapter to verify upload completion for.
        chapter_id: String,
        /// User that scheduled the check.
        actor_user_id: String,
    },
}

impl ChapterPayload {
    /// Selects an existing queue for this chapter operation.
    pub const fn topic(&self) -> Topic {
        //
        match self {
            Self::TryAdvanceRawProvideStage { .. } => Topic::Chapter,
        }
    }
}
