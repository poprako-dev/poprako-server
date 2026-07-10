//! Payload types for chapter domain events.

use crate::value::chapter::Stage;

/// Payload for the [`ChapterPublished`] event.
///
/// [`ChapterPublished`]: crate::part::effect::event::Event::ChapterPublished
pub struct ChapterPublishedPayload {
    pub chapter_id: String,
}

/// Payload for the [`ChapterWorkflowCompleted`] event.
///
/// [`ChapterWorkflowCompleted`]: crate::part::effect::event::Event::ChapterWorkflowCompleted
pub struct ChapterWorkflowCompletedPayload {
    pub chapter_id: String,
    pub completed_stage: Stage,
}
