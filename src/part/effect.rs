use std::iter::Once;
use std::vec::IntoIter;

use async_trait::async_trait;

use crate::part::effect::event::Event;

pub mod event;

pub trait EventIter {
    type Iter: Iterator<Item = Event>;

    fn into_iter(self) -> Self::Iter;
}

impl EventIter for Vec<Event> {
    type Iter = IntoIter<Event>;

    fn into_iter(self) -> Self::Iter {
        <Vec<Event> as IntoIterator>::into_iter(self)
    }
}

impl EventIter for Event {
    type Iter = Once<Event>;

    fn into_iter(self) -> Self::Iter {
        std::iter::once(self)
    }
}

#[async_trait]
pub trait EffectDevelop {
    async fn develop<I>(&self, iter: I)
    where
        I: EventIter + Send;
}

#[async_trait]
pub trait EffectEmit<D>
where
    D: EffectDevelop + Send + Sync,
{
    async fn emit(self, develop: &D);
}

#[async_trait]
impl<T, D> EffectEmit<D> for T
where
    T: EventIter + Send,
    D: EffectDevelop + Send + Sync,
{
    async fn emit(self, develop: &D) {
        develop.develop(self).await;
    }
}
