use async_trait::async_trait;

use crate::part::effect::event::Event;

pub mod event;

pub trait EffectIter {
    type Iter: Iterator<Item = Event>;

    fn into_iter(self) -> Self::Iter;
}

impl EffectIter for Vec<Event> {
    type Iter = std::vec::IntoIter<Event>;

    fn into_iter(self) -> Self::Iter {
        <Vec<Event> as IntoIterator>::into_iter(self)
    }
}

#[async_trait]
pub trait EffectHandler {
    async fn handle<I>(&self, iter: I)
    where
        I: EffectIter + Send;
}
