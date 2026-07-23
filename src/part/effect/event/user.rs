//! Payload types for domain events.

/// Payload for the [`UserActive`] event.
///
/// [`UserActive`]: crate::part::effect::event::Event::UserActive
pub struct UserActivePayload {
    /// Unique identifier of the user who became active.
    pub user_id: String,
}

/// Payload for the [`UserSignedUp`] event.
///
/// [`UserSignedUp`]: crate::part::effect::event::Event::UserSignedUp
pub struct UserSignedUpPayload {
    /// Identifier of the team the new user was invited to.
    pub team_id: String,
    /// Identifier of the user who sent the invitation.
    pub invitor_id: String,
    /// Qualified identifier of the newly signed-up user.
    pub invitee_qid: String,
}
