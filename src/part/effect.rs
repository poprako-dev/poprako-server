//! Side-effect port for domain event dispatch.
//!
//! Use cases emit [`Event`] values during execution. Implementations of
//! [`EffectDevelop`] process these events — for example, logging analytics,
//! sending push notifications, or updating caches.
//!
//! The dispatch path uses two traits:
//!
//! 1. **[`EventIter`]** — converts an event-bearing type into an iterator
//!    of [`Event`] values. Implemented for both single events and buffers.
//! 2. **[`EffectDevelop`]** — the port implementation that receives and
//!    processes the event iterator.

use std::future::Future;
use std::iter::Once;
use std::vec::IntoIter;

use crate::part::effect::event::Event;

/// Domain event types.
pub mod event;

/// Trait for types that can yield an iterator of [`Event`] values.
///
/// Implemented for [`Event`] (yielding a single-element iterator) and
/// [`Vec<Event>`](Vec) (yielding a multi-element iterator), enabling
/// both single-event and batched dispatch through [`EffectDevelop`].
pub trait EventIter {
    /// The iterator type yielded by [`into_iter`](EventIter::into_iter).
    type Iter: Iterator<Item = Event>;

    /// Consumes self and returns an iterator of [`Event`] values.
    fn into_iter(self) -> Self::Iter;
}

impl EventIter for Vec<Event> {
    // Event iterator for a pre-collected vector of side effects.
    type Iter = IntoIter<Event>;

    // Consumes the vector and yields its events one by one.
    fn into_iter(self) -> Self::Iter {
        <Vec<Event> as IntoIterator>::into_iter(self)
    }
}

impl EventIter for Event {
    // Single-element event iterator wrapping one domain event.
    type Iter = Once<Event>;

    // Consumes the event and yields it as a single-element iterator.
    fn into_iter(self) -> Self::Iter {
        std::iter::once(self)
    }
}

/// Port for processing domain events.
///
/// Implementations receive an iterator of [`Event`] values and dispatch
/// them to the appropriate side-effect handlers (logging, analytics,
/// notifications, etc.).
pub trait EffectDevelop {
    /// Dispatches each event in the provided iterator to the appropriate side-effect handlers.
    fn develop<I>(&self, iter: I) -> impl Future<Output = ()> + Send
    where
        I: EventIter + Send;
}
