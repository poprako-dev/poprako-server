mod user;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use crossfire::mpsc;
use crossfire::oneshot::{self, RxOneshot, TxOneshot};
use crossfire::{AsyncRx, MAsyncTx, TrySendError};
use tracing::Level;
use tracing::instrument;

use crate::api::harness::HarnessInner;
use crate::domain::effect::EffectSink;
use crate::domain::model::event::{Event, EventEmit};
use crate::infrastructure::effect::user::notify_invitor_handler;

/// An async effect sink that dispatches domain events to hardcoded handlers
/// via a background task.
///
/// Uses an mpsc channel for event queuing and a pair of oneshot channels for
/// graceful shutdown with drain: on [`close`](Self::close), it stops accepting
/// new events, then processes all remaining events in the channel before
/// returning.
pub struct AsyncEffectSink {
    accepting: Arc<AtomicBool>,

    inlet: MAsyncTx<mpsc::Array<Event>>,

    shutdown: Mutex<Option<TxOneshot<()>>>,
    done: Mutex<Option<RxOneshot<()>>>,
}

impl AsyncEffectSink {
    /// Creates a new `AsyncEffectSink` and spawns a background task.
    ///
    /// The background task receives events from the mpsc channel and dispatches
    /// them to hardcoded effect handlers. The given `harn` is cloned for use
    /// by the background task.
    pub fn new(harn: Arc<HarnessInner>, buffer: usize) -> Self {
        let (inlet, outlet) = mpsc::bounded_async(buffer);

        let (shutdown_inlet, shutdown_outlet) = oneshot::oneshot();
        let (done_inlet, done_outlet) = oneshot::oneshot();

        let accepting = Arc::new(AtomicBool::new(true));
        let accepting_task = Arc::clone(&accepting);

        let harn_task = Arc::clone(&harn);

        tokio::spawn(async move {
            handle_task(
                harn_task,
                outlet,
                shutdown_outlet,
                done_inlet,
                accepting_task,
            )
            .await;
        });

        Self {
            inlet,
            shutdown: Mutex::new(Some(shutdown_inlet)),
            done: Mutex::new(Some(done_outlet)),
            accepting,
        }
    }

    /// Signals the background task to shut down, then waits for it to drain
    /// all remaining events and exit.
    ///
    /// Idempotent: subsequent calls return immediately via the atomic
    /// `accepting` flag.
    pub async fn close(&self) {
        // Fast path: already closed.
        if !self.accepting.swap(false, Ordering::AcqRel) {
            return;
        }

        // Signal the background task to begin shutdown.
        if let Some(inlet) = self.shutdown.lock().unwrap().take() {
            inlet.send(());
        }

        // Wait for the background task to finish draining.
        let Some(outlet) = self.done.lock().unwrap().take() else {
            return;
        };

        let _ = outlet.await;
    }
}

#[async_trait]
impl EffectSink for AsyncEffectSink {
    async fn handle<E>(&self, src: &mut E)
    where
        E: EventEmit + Send,
    {
        // Fast path: already shutting down — silently drop.
        if !self.accepting.load(Ordering::Acquire) {
            return;
        }

        for event in src.pull_events() {
            match self.inlet.try_send(event) {
                Ok(()) => {}
                Err(TrySendError::Full(ev)) => {
                    tracing::warn!(
                        event_type = ?ev.event_type(),
                        "[AsyncEffectSink::handle] event sink full, dropping event",
                    );
                }
                Err(TrySendError::Disconnected(_)) => {
                    tracing::warn!(
                        "[AsyncEffectSink::handle] event sink disconnected, dropping remaining events",
                    );
                    break;
                }
            }
        }
    }
}

// ── Background task ────────────────────────────────────────────────────────

#[instrument(skip_all, level = Level::DEBUG)]
async fn handle_task(
    harn: Arc<HarnessInner>,
    outlet: AsyncRx<mpsc::Array<Event>>,
    shutdown_outlet: RxOneshot<()>,
    done_inlet: TxOneshot<()>,
    accepting: Arc<AtomicBool>,
) {
    // Main loop: receive and dispatch events.
    tokio::pin!(shutdown_outlet);

    loop {
        tokio::select! {
            result = outlet.recv() => {
                match result {
                    Ok(event) => dispatch(&harn, event).await,
                    Err(_) => break, // channel closed — all senders dropped
                }
            }
            _ = &mut shutdown_outlet => {
                // Stop accepting new events (handle() checks this flag).
                accepting.store(false, Ordering::Release);
                break;
            }
        }
    }

    // Drain: process any events that were already in the channel.
    while let Ok(event) = outlet.try_recv() {
        dispatch(&harn, event).await;
    }

    // Signal that drain is complete.
    done_inlet.send(());
}

// ── Dispatch ───────────────────────────────────────────────────────────────

/// Dispatches a domain event to the appropriate hardcoded effect handler.
async fn dispatch(harn: &HarnessInner, event: Event) {
    match event {
        Event::UserSignedUp(payload) => {
            notify_invitor_handler(harn, payload).await;
        }
    }
}
