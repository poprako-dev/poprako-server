//! Event bus port for domain event dispatch.
//!
//! Use cases emit [`Event`] values during execution. Implementations of
//! [`EventBus`] process these events — for example, logging analytics,
//! sending push notifications, or updating caches.
//!
//! The dispatch path uses two traits:
//!
//! 1. **[`EffectEvent`]** — converts an [`Event`] or its batch into an
//!    iterator of [`Event`] values.
//! 2. **[`EventBus`]** — the port implementation that receives and
//!    processes the event iterator.

/// Domain event types.
pub mod event;

use std::future::Future;

use crate::part::effect::event::Event;

/// Trait for values that can be dispatched through [`EventBus`].
///
/// Implementations convert themselves into one or more [`Event`] values.
/// Construct the appropriate [`Event`] variant before calling
/// [`emit_on`](EffectEvent::emit_on).
pub trait EffectEvent {
    /// Consumes self and returns its events.
    fn into_events(self) -> impl IntoIterator<Item = Event> + Send;

    /// Emits this event through `bus`.
    fn emit_on<B>(self, bus: &B) -> impl Future<Output = ()>
    where
        Self: Sized,
        B: EventBus + ?Sized,
    {
        bus.emit(self.into_events())
    }
}

impl EffectEvent for Vec<Event> {
    // Returns the pre-collected side effects.
    fn into_events(self) -> impl IntoIterator<Item = Event> + Send {
        self
    }
}

impl EffectEvent for Event {
    // Dispatches one event without allocating a batch buffer.
    fn into_events(self) -> impl IntoIterator<Item = Self> + Send {
        std::iter::once(self)
    }
}

/// Port for processing domain events.
///
/// Implementations receive an iterator of [`Event`] values and dispatch
/// them to the appropriate side-effect handlers (logging, analytics,
/// notifications, etc.).
pub trait EventBus {
    /// Emits each event in the provided iterator to the appropriate side-effect handlers.
    fn emit<I>(&self, events: I) -> impl Future<Output = ()>
    where
        I: IntoIterator<Item = Event>;
}
