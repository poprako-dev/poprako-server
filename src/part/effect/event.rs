use std::vec::IntoIter;

use crate::part::effect::EffectIter;
use crate::part::effect::event::user::{UserActivePayload, UserSignedUpPayload};

pub mod user;

pub enum Event {
    UserActive(UserActivePayload),
    UserSignedUp(UserSignedUpPayload),
}

#[derive(Default)]
pub struct EventBuffer {
    events: Vec<Event>,
}

impl EventBuffer {
    pub fn push(&mut self, event: Event) {
        self.events.push(event);
    }
}

impl EffectIter for EventBuffer {
    type Iter = IntoIter<Event>;

    fn into_iter(self) -> Self::Iter {
        <Vec<Event> as IntoIterator>::into_iter(self.events)
    }
}
