pub mod user;

use user::*;

// DomainEventType is used for a lightweight check for event sink and handler.
#[derive(Debug)]
pub enum EventType {
    UserSignedUp,
}

#[derive(Clone, Debug)]
pub enum Event {
    UserSignedUp(UserSignedUpEvent),
}

impl Event {
    pub fn event_type(&self) -> EventType {
        match self {
            Event::UserSignedUp(_) => EventType::UserSignedUp,
        }
    }
}

/// Accumulates domain events during a business operation.
///
/// Implemented by input aggregates (e.g. [`UserForm`](crate::domain::model::aggregate::user::UserForm))
/// that carry a private `events: Vec<DomainEvent>` field. The usecase layer pushes events
/// into the form before passing it to the query layer's `create`.
pub trait EventSink {
    /// Appends a domain event to the internal buffer.
    fn push_event(&mut self, event: Event);
}

/// Drains all accumulated domain events from an input aggregate.
///
/// Called **after** a successful transaction commit so events can be
/// published to the event bus. The internal buffer is cleared.
pub trait EventEmit {
    /// Takes all pending domain events out and leaves the buffer empty.
    fn pull_events(&mut self) -> Vec<Event>;
}
