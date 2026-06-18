use std::iter::Once;
use std::vec::IntoIter;

use async_trait::async_trait;

use crate::part::effect::event::Event;

pub mod event;

pub trait EffectIter {
    type Iter: Iterator<Item = Event>;

    fn into_iter(self) -> Self::Iter;
}

impl EffectIter for Vec<Event> {
    type Iter = IntoIter<Event>;

    fn into_iter(self) -> Self::Iter {
        <Vec<Event> as IntoIterator>::into_iter(self)
    }
}

impl EffectIter for Event {
    type Iter = Once<Event>;

    fn into_iter(self) -> Self::Iter {
        std::iter::once(self)
    }
}

#[async_trait]
pub trait Develop {
    async fn develop<I>(&self, iter: I)
    where
        I: EffectIter + Send;
}

#[async_trait]
pub trait EffectEmit<D>
where
    D: Develop + Send + Sync,
{
    async fn emit(self, develop: &D);
}

#[async_trait]
impl<T, D> EffectEmit<D> for T
where
    T: EffectIter + Send,
    D: Develop + Send + Sync,
{
    async fn emit(self, develop: &D) {
        develop.develop(self).await;
    }
}
