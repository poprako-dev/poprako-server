use crate::part::effect::EffectIter;

pub enum Event {}

pub struct EventBuffer {
    events: Vec<Event>,
}

impl EventBuffer {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn push(&mut self, event: Event) {
        self.events.push(event);
    }
}

impl EffectIter for EventBuffer {
    type Iter = std::vec::IntoIter<Event>;

    fn into_iter(self) -> Self::Iter {
        <Vec<Event> as IntoIterator>::into_iter(self.events)
    }
}
