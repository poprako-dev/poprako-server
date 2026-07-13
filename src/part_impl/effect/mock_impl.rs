//! Mock implementation of [EffectDevelop] for testing event collection.

use crate::part::effect::{EffectDevelop, EventIter};
use crate::part_impl::repo::mock_impl::Mock;

/// Mock implementation of [EffectDevelop].
///
/// Collected events are stored in the mock's internal event buffer and can
/// be drained via [Mock::drain_events] for assertion.
impl EffectDevelop for Mock {
    async fn develop<I>(&self, iter: I)
    where
        I: EventIter + Send,
    {
        self.events.lock().unwrap().extend(iter.into_iter());
    }
}

// develop_collects_events(Develop::develop)(positive): emitted events should be stored for later draining.

use crate::part::effect::event::Event;
use crate::part::effect::event::user::UserActivePayload;

/// Mock helper that verifies events are collected for later draining.
#[tokio::test]
async fn develop_collects_events() {
    //
    let mock = Mock::new();

    EffectDevelop::develop(
        &mock,
        Event::UserActive(UserActivePayload {
            user_id: "user-1".into(),
        }),
    )
    .await;

    assert_eq!(mock.event_count(), 1);

    let events = mock.drain_events();

    assert_eq!(events.len(), 1);

    assert_eq!(mock.event_count(), 0);
}
