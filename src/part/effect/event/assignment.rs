//! Payload types for assignment domain events.

/// Payload for the [`AssignmentCreated`] event.
///
/// [`AssignmentCreated`]: crate::part::effect::event::Event::AssignmentCreated
pub struct AssignmentCreatedPayload {
    pub user_id: String,
    pub chapter_id: String,
}

/// Payload for the [`AssignmentRemoved`] event.
///
/// [`AssignmentRemoved`]: crate::part::effect::event::Event::AssignmentRemoved
pub struct AssignmentRemovedPayload {
    pub user_id: String,
    pub chapter_id: String,
    pub was_published: bool,
}
