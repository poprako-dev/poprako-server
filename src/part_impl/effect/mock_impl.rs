#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

//! Mock implementation of [EventBus] for testing event collection.

use crate::part::effect::event::Event;
use crate::part::effect::event::user::UserSignedUpEvent;
use crate::part::effect::{EffectEvent, EventBus};
use crate::part_impl::repo::mock_impl::Mock;

/// Mock implementation of [`EventBus`].
///
/// Collected events are stored in the mock's internal event buffer and can
/// be drained via [`Mock::drain_events`] for assertion.
impl EventBus for Mock {
    // Collect emitted events into the mock event buffer during tests.
    async fn emit(&self, events: Vec<Event>) {
        self.events.lock().unwrap().extend(events);
    }
}

// emit_collects_events(EventBus::emit)(positive): emitted events should be stored for later draining.

/// Mock helper that verifies events are collected for later draining.
#[tokio::test]
async fn emit_collects_events() {
    //
    let mock = Mock::new();

    Event::UserSignedUp {
        payload: UserSignedUpEvent {
            team_id: "team--00000000001".into(),
            invitor_id: "user--00000000001".into(),
            invitee_qid: "qid-2".into(),
        },
    }
    .emit_on(&mock)
    .await;

    assert_eq!(mock.event_count(), 1);

    let events = mock.drain_events();

    assert_eq!(events.len(), 1);

    assert_eq!(mock.event_count(), 0);
}
