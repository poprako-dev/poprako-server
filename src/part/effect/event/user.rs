//! Payload types for domain events.

/// Payload for the [`UserActive`] event.
///
/// [`UserActive`]: crate::part::effect::event::Event::UserActive
pub struct UserActivePayload {
    pub user_id: String,
}

/// Payload for the [`UserSignedUp`] event.
///
/// [`UserSignedUp`]: crate::part::effect::event::Event::UserSignedUp
pub struct UserSignedUpPayload {
    pub team_id: String,
    pub invitor_id: String,
    pub invitee_qid: String,
}
