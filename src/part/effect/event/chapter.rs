//! Chapter domain events.
use crate::value::chapter::Stage;

/// Event emitted when a chapter reaches publish completion.
///
/// [`ChapterPublished`]: crate::part::effect::event::Event::ChapterPublished
pub struct ChapterPublishedEvent {
    /// Unique identifier of the published chapter.
    pub chapter_id: String,
}

/// Event emitted when a chapter workflow stage reaches completion.
///
/// [`ChapterWorkflowCompleted`]: crate::part::effect::event::Event::ChapterWorkflowCompleted
pub struct ChapterWorkflowCompletedEvent {
    //
    /// Unique identifier of the chapter whose workflow completed.
    pub chapter_id: String,
    /// The final stage that was reached in the completed workflow.
    pub completed_stage: Stage,
}
