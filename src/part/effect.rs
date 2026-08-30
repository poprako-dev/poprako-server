//! Side-effect port for domain event dispatch.
//!
//! Use cases emit [`Event`] values during execution. Implementations of
//! [`EffectDevelop`] process these events — for example, logging analytics,
//! sending push notifications, or updating caches.
//!
//! The dispatch path uses two traits:
//!
//! 1. **[`EffectEvent`]** — converts an [`Event`] or its batch into an
//!    iterator of [`Event`] values.
//! 2. **[`EffectDevelop`]** — the port implementation that receives and
//!    processes the event iterator.

/// Domain event types.
pub mod event;

use std::future::Future;

use crate::part::effect::event::Event;

/// Trait for values that can be dispatched through [`EffectDevelop`].
///
/// Implementations convert themselves into one or more [`Event`] values.
/// Construct the appropriate [`Event`] variant before calling
/// [`develop_on`](EffectEvent::develop_on).
pub trait EffectEvent {
    /// Consumes self and returns its events.
    fn into_events(self) -> Vec<Event>;

    /// Dispatches this event through `develop`.
    fn develop_on<D>(self, develop: &D) -> impl Future<Output = ()>
    where
        Self: Sized,
        D: Develop + ?Sized,
    {
        develop.develop(self.into_events())
    }
}

impl EffectEvent for Vec<Event> {
    // Returns the pre-collected side effects.
    fn into_events(self) -> Vec<Event> {
        self
    }
}

impl EffectEvent for Event {
    // Wraps one event for dispatch.
    fn into_events(self) -> Vec<Event> {
        vec![self]
    }
}

/// Port for processing domain events.
///
/// Implementations receive an iterator of [`Event`] values and dispatch
/// them to the appropriate side-effect handlers (logging, analytics,
/// notifications, etc.).
pub trait Develop {
    /// Dispatches each event in the provided iterator to the appropriate side-effect handlers.
    fn develop(&self, events: Vec<Event>) -> impl Future<Output = ()> + Send;
}
