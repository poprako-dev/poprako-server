//! Domain event types emitted during use case execution.

use crate::part::effect::event::chapter::{
    ChapterPublishedPayload, ChapterWorkflowCompletedPayload,
};
use crate::part::effect::event::user::{UserActivePayload, UserSignedUpPayload};

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
    /// Emitted when a chapter reaches publish completion.
    ChapterPublished(ChapterPublishedPayload),
    /// Emitted when one chapter workflow stage reaches completion.
    ChapterWorkflowCompleted(ChapterWorkflowCompletedPayload),
}
