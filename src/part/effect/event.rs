//! Domain event types emitted during use case execution.

use crate::part::effect::event::assignment::{AssignmentCreatedPayload, AssignmentRemovedPayload};
use crate::part::effect::event::chapter::{
    ChapterPublishedPayload, ChapterRemovedPayload, ChapterWorkflowCompletedPayload,
    ChapterWorkflowRevertedPayload,
};
use crate::part::effect::event::user::{UserActivePayload, UserSignedUpPayload};

pub mod assignment;
pub mod chapter;
pub mod user;

/// Domain events produced by use cases and dispatched through [`EffectDevelop`].
///
/// Each variant carries a payload struct with the data relevant to that event.
///
/// [`EffectDevelop`]: crate::part::effect::EffectDevelop
pub enum Event {
    /// Emitted when a user shows activity (e.g., views their own profile).
    UserActive(UserActivePayload),
    /// Emitted when a new user signs up via an invitation.
    UserSignedUp(UserSignedUpPayload),
    /// Emitted when an assignment is created.
    AssignmentCreated(AssignmentCreatedPayload),
    /// Emitted when an assignment is removed.
    AssignmentRemoved(AssignmentRemovedPayload),
    /// Emitted when a chapter reaches publish completion.
    ChapterPublished(ChapterPublishedPayload),
    /// Emitted when one chapter workflow stage reaches completion.
    ChapterWorkflowCompleted(ChapterWorkflowCompletedPayload),
    /// Emitted when one chapter workflow stage is reverted.
    ChapterWorkflowReverted(ChapterWorkflowRevertedPayload),
    /// Emitted when a chapter is removed.
    ChapterRemoved(ChapterRemovedPayload),
}
