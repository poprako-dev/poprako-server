//! Payload types for chapter domain events.

use crate::value::chapter::Stage;

/// Payload for the [`ChapterPublished`] event.
///
/// [`ChapterPublished`]: crate::part::effect::event::Event::ChapterPublished
pub struct ChapterPublishedPayload {
    /// Unique identifier of the published chapter.
    pub chapter_id: String,
}

/// Payload for the [`ChapterWorkflowCompleted`] event.
///
/// [`ChapterWorkflowCompleted`]: crate::part::effect::event::Event::ChapterWorkflowCompleted
pub struct ChapterWorkflowCompletedPayload {
    /// Unique identifier of the chapter whose workflow completed.
    pub chapter_id: String,
    /// The final stage that was reached in the completed workflow.
    pub completed_stage: Stage,
}
