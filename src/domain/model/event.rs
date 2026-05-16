pub mod user;

use user::*;

use async_trait::async_trait;

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

pub trait EventBuffer {
    // push_event pushes a domain event to the event root.
    fn push_event(&mut self, event: DomainEvent);
}

pub trait EventEmit {
    // pull_events pulls all the domain events from the event root, and clears the event source.
    fn pull_events(&mut self) -> Vec<DomainEvent>;
}

#[async_trait]
pub trait EventHandler {
    // interested_events returns the domain event types that the handler is interested in.
    fn interested_events(&self) -> Vec<DomainEventType>;

    // handle handles a domain event, and may produce new domain events.
    // NOTE: It will ignore silently the event if it is not interested in it.
    async fn handle(&mut self, event: DomainEvent);
}
