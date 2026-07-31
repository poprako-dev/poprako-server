//! Side-effect port for domain event dispatch.
//!
//! Use cases emit [`Event`] values during execution. Implementations of
//! [`EffectDevelop`] process these events — for example, logging analytics,
//! sending push notifications, or updating caches.
//!
//! The dispatch path uses two traits:
//!
//! 1. **[`EffectEvent`]** — converts an event-bearing type into an iterator
//!    of [`Event`] values. Implemented for both single events and buffers.
//! 2. **[`EffectDevelop`]** — the port implementation that receives and
//!    processes the event iterator.

use std::future::Future;
use std::iter::Once;
use std::vec::IntoIter;

use crate::part::effect::event::Event;
use crate::part::effect::event::chapter::{
    ChapterPublishedEvent, ChapterWorkflowCompletedEvent,
};
use crate::part::effect::event::user::{UserActiveEvent, UserSignedUpEvent};

/// Domain event types.
pub mod event;

/// Trait for values that can be dispatched through [`EffectDevelop`].
///
/// Implementations convert themselves into one or more [`Event`] values.
/// Concrete event structs use [`develop_on`](EffectEvent::develop_on) to
/// dispatch themselves directly through an effect port.
pub trait EffectEvent {
    /// The iterator type yielded by [`into_iter`](EffectEvent::into_iter).
    type Iter: Iterator<Item = Event>;

    /// Consumes self and returns an iterator of [`Event`] values.
    fn into_iter(self) -> Self::Iter;

    /// Dispatches this event through `develop`.
    fn develop_on<D>(self, develop: &D) -> impl Future<Output = ()> + Send
    where
        Self: Sized + Send,
        D: EffectDevelop + ?Sized,
    {
        develop.develop(self)
    }
}

impl EffectEvent for Vec<Event> {
    // Event iterator for a pre-collected vector of side effects.
    type Iter = IntoIter<Event>;

    // Consumes the vector and yields its events one by one.
    fn into_iter(self) -> Self::Iter {
        <Vec<Event> as IntoIterator>::into_iter(self)
    }
}

impl EffectEvent for Event {
    // Single-element event iterator wrapping one domain event.
    type Iter = Once<Event>;

    // Consumes the event and yields it as a single-element iterator.
    fn into_iter(self) -> Self::Iter {
        std::iter::once(self)
    }
}

impl EffectEvent for UserActiveEvent {
    // One activity event becomes one dispatched event.
    type Iter = Once<Event>;

    // Wraps this typed event in the internal dispatch enum.
    fn into_iter(self) -> Self::Iter {
        std::iter::once(Event::UserActive(self))
    }
}

impl EffectEvent for UserSignedUpEvent {
    // One signup event becomes one dispatched event.
    type Iter = Once<Event>;

    // Wraps this typed event in the internal dispatch enum.
    fn into_iter(self) -> Self::Iter {
        std::iter::once(Event::UserSignedUp(self))
    }
}

impl EffectEvent for ChapterPublishedEvent {
    // One chapter-publication event becomes one dispatched event.
    type Iter = Once<Event>;

    // Wraps this typed event in the internal dispatch enum.
    fn into_iter(self) -> Self::Iter {
        std::iter::once(Event::ChapterPublished(self))
    }
}

impl EffectEvent for ChapterWorkflowCompletedEvent {
    // One workflow-completion event becomes one dispatched event.
    type Iter = Once<Event>;

    // Wraps this typed event in the internal dispatch enum.
    fn into_iter(self) -> Self::Iter {
        std::iter::once(Event::ChapterWorkflowCompleted(self))
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
        I: EffectEvent + Send;
}
