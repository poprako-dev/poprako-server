mod user;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use crossfire::mpsc::{Array, bounded_async};
use crossfire::oneshot::{self, RxOneshot, TxOneshot};
use crossfire::{AsyncRx, MAsyncTx, TrySendError};
use tracing::Level;
use tracing::instrument;

use crate::domain::effect::EffectSink;
use crate::domain::model::event::{Event, EventEmit};
use crate::harness::HarnessBase;
use crate::infra::effect::user::notify_invitor_handler;

/// An async effect sink that dispatches domain events to hardcoded handlers
/// via a background task.
///
/// Uses an mpsc channel for event queuing and a pair of oneshot channels for
/// graceful shutdown with drain: on [`close`](Self::close), it stops accepting
/// new events, then processes all remaining events in the channel before
/// returning.
pub struct AsyncEffectSink {
    accepting: Arc<AtomicBool>,

    // TODO: is a masyntx really necessary?
    send: MAsyncTx<Array<Event>>,

    shutdown: Mutex<Option<TxOneshot<()>>>,
    done: Mutex<Option<RxOneshot<()>>>,
}

pub type SharedEffectSink = Arc<AsyncEffectSink>;

impl AsyncEffectSink {
    /// Creates a new `AsyncEffectSink` and spawns a background task.
    ///
    /// The background task receives events from the mpsc channel and dispatches
    /// them to hardcoded effect handlers. The given `harn` is cloned for use
    /// by the background task.
    pub fn new(harn: Arc<HarnessBase>, buffer: usize) -> Self {
        let (send, recv) = bounded_async(buffer);

        let (shutdown_send, shutdown_recv) = oneshot::oneshot();
        let (done_send, done_recv) = oneshot::oneshot();

        let accepting = Arc::new(AtomicBool::new(true));
        let accepting_task = Arc::clone(&accepting);

        let harn_task = Arc::clone(&harn);

        tokio::spawn(async move {
            handle_task(harn_task, recv, shutdown_recv, done_send, accepting_task).await;
        });

        Self {
            send,
            shutdown: Mutex::new(Some(shutdown_send)),
            done: Mutex::new(Some(done_recv)),
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
        if let Some(send) = self.shutdown.lock().unwrap().take() {
            send.send(());
        }

        // Wait for the background task to finish draining.
        let Some(recv) = self.done.lock().unwrap().take() else {
            return;
        };

        let _ = recv.await;
    }
}

pub fn shared_effect_sink(harn: Arc<HarnessBase>, buffer: usize) -> SharedEffectSink {
    Arc::new(AsyncEffectSink::new(harn, buffer))
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
            match self.send.try_send(event) {
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
                _ => {}
            }
        }
    }
}

// ── Background task ────────────────────────────────────────────────────────

#[instrument(skip_all, level = Level::DEBUG)]
async fn handle_task(
    harn: Arc<HarnessBase>,
    recv: AsyncRx<Array<Event>>,
    shutdown_recv: RxOneshot<()>,
    done_send: TxOneshot<()>,
    accepting: Arc<AtomicBool>,
) {
    // Main loop: receive and dispatch events.
    tokio::pin!(shutdown_recv);

    loop {
        tokio::select! {
            result = recv.recv() => {
                match result {
                    Ok(event) => dispatch(&harn, event).await,
                    Err(_) => break, // channel closed — all senders dropped
                }
            }
            _ = &mut shutdown_recv => {
                // Stop accepting new events (handle() checks this flag).
                accepting.store(false, Ordering::Release);
                break;
            }
        }
    }

    // Drain: process any events that were already in the channel.
    while let Ok(event) = recv.try_recv() {
        dispatch(&harn, event).await;
    }

    // Signal that drain is complete.
    done_send.send(());
}

// ── Dispatch ───────────────────────────────────────────────────────────────

/// Dispatches a domain event to the appropriate hardcoded effect handler.
async fn dispatch(harn: &HarnessBase, event: Event) {
    match event {
        Event::UserSignedUp(payload) => {
            notify_invitor_handler(harn, payload).await;
        }
    }
}
