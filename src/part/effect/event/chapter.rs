//! Payload types for chapter domain events.

use crate::value::chapter::WorkflowStage;

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
    pub completed_stage: WorkflowStage,
}

/// Payload for the [`ChapterWorkflowReverted`] event.
///
/// [`ChapterWorkflowReverted`]: crate::part::effect::event::Event::ChapterWorkflowReverted
pub struct ChapterWorkflowRevertedPayload {
    pub chapter_id: String,
    pub reverted_stage: WorkflowStage,
}

/// Payload for the [`ChapterRemoved`] event.
///
/// [`ChapterRemoved`]: crate::part::effect::event::Event::ChapterRemoved
pub struct ChapterRemovedPayload {
    pub chapter_id: String,
    pub was_published: bool,
    pub assigned_user_ids: Vec<String>,
}
