//! Side-effect port for domain event dispatch.
//!
//! Use cases emit [`Event`] values during execution. Implementations of
//! [`EffectDevelop`] process these events — for example, logging analytics,
//! sending push notifications, or updating caches.
//!
//! The emission path follows a three-trait chain:
//!
//! 1. **[`EventIter`]** — converts an event-bearing type into an iterator
//!    of [`Event`] values. Implemented for both single events and buffers.
//! 2. **[`EffectEmit`]** — provides the `.emit(developer)` ergonomic method
//!    on anything that implements [`EventIter`].
//! 3. **[`EffectDevelop`]** — the port implementation that receives and
//!    processes the event iterator.

use std::iter::Once;
use std::vec::IntoIter;

use async_trait::async_trait;

use crate::part::effect::event::Event;

/// Domain event types.
pub mod event;

/// Trait for types that can yield an iterator of [`Event`] values.
///
/// Implemented for [`Event`] (yielding a single-element iterator) and
/// [`Vec<Event>`](Vec) (yielding a multi-element iterator), enabling
/// both single-event and batched emission through the same
/// [`EffectEmit`] interface.
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

/// Port for processing domain events.
///
/// Implementations receive an iterator of [`Event`] values and dispatch
/// them to the appropriate side-effect handlers (logging, analytics,
/// notifications, etc.).
#[async_trait]
pub trait EffectDevelop {
    async fn develop<I>(&self, iter: I)
    where
        I: EventIter + Send;
}

/// Ergonomic emission method for anything that implements [`EventIter`].
///
/// Blanket-implemented for all `T: EventIter + Send` against any
/// `D: EffectDevelop + Send + Sync`. Calling `.emit(developer)` is
/// equivalent to `developer.develop(self)`.
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
