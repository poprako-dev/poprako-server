//! Deferred-action task data.

use std::time::Duration;

/// Data required to persist one deferred action.
pub struct Task<'a, I, P>
where
    I: AsRef<str>,
    P: ?Sized,
{
    /// Stable task identity.
    pub id: &'a I,
    /// Deferred payload.
    pub payload: &'a P,
    /// Minimum delay before delivery eligibility.
    pub delay: Option<Duration>,
}
