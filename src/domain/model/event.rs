pub mod user;

use user::*;

// DomainEventType is used for a lightweight check for event sink and handler.
pub enum DomainEventType {
    UserRegistered,
}

pub enum DomainEvent {
    UserRegistered(UserRegisteredEvent),
}

impl DomainEvent {
    pub fn event_type(&self) -> DomainEventType {
        match self {
            DomainEvent::UserRegistered(_) => DomainEventType::UserRegistered,
        }
    }
}

pub trait EventSink {
    // push_event pushes a domain event to the event root.
    fn push_event(&mut self, event: DomainEvent);
}

pub trait EventEmit {
    // pull_events pulls all the domain events from the event root, and clears the event source.
    fn pull_events(&mut self) -> Vec<DomainEvent>;
}
