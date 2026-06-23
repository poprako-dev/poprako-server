//! Domain event types emitted during use case execution.

use std::vec::IntoIter;

use crate::part::effect::EventIter;
use crate::part::effect::event::user::{UserActivePayload, UserSignedUpPayload};

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
}

/// Collects [`Event`] values during a use case and flushes them in batch.
///
/// Events are pushed into the buffer as the use case executes. After the
/// transaction commits, the buffer is drained into [`EffectDevelop::develop`]
/// via the [`EffectEmit::emit`] blanket implementation.
///
/// [`EffectDevelop::develop`]: crate::part::effect::EffectDevelop::develop
/// [`EffectEmit::emit`]: crate::part::effect::EffectEmit::emit
#[derive(Default)]
pub struct EventBuffer {
    events: Vec<Event>,
}

impl EventBuffer {
    /// Appends an event to the buffer.
    pub fn push(&mut self, event: Event) {
        self.events.push(event);
    }
}

impl EventIter for EventBuffer {
    type Iter = IntoIter<Event>;

    fn into_iter(self) -> Self::Iter {
        <Vec<Event> as IntoIterator>::into_iter(self.events)
    }
}
