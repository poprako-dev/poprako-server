//! Async background dispatcher for side-effect events.

use std::sync::Arc;

use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use crate::part::effect::event::Event;
use crate::part::effect::{EffectDevelop, EventIter};
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::system_mail::SystemMailRepo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::user::UserRepo;

/// Chapter event handlers.
mod chapter;
/// Event dispatch logic.
mod dispatch;
/// Background event handler runner.
mod handler;
#[cfg(test)]
mod tests;
/// User event handlers.
mod user;

/// Async side-effect dispatcher backed by a bounded channel.
///
/// Spawns a background handler on construction that drains the queue and
/// dispatches each event to the appropriate domain handler. Use
/// [`close`](AsyncEffectDevelop::close) before dropping to drain pending
/// events gracefully.
pub struct AsyncEffectDevelop {
    /// Bounded channel sender for enqueueing events.
    send: Sender<Event>,
    /// Cancellation token to signal graceful shutdown.
    token: CancellationToken,
    /// Watch receiver that signals when background processing completes.
    done: watch::Receiver<bool>,
}

impl AsyncEffectDevelop {
    /// Creates a dispatcher and starts its background task.
    pub fn new<C, R>(repo: Arc<R>, buffer_size: usize) -> Self
    where
        C: Send + 'static,
        R: AssignmentRepo<C>
            + ChapterRepo<C>
            + TeamRepo<C>
            + SystemMailRepo<C>
            + UserRepo<C>
            + Send
            + Sync
            + 'static,
    {
        let (send, recv) = tokio::sync::mpsc::channel(buffer_size);

        let token = CancellationToken::new();

        let (done_send, done) = watch::channel(false);

        let handler =
            handler::EffectHandler::<R>::new(repo, recv, token.clone());

        tokio::spawn(async move {
            //
            handler.run().await;

            done_send.send_replace(true);
        });

        Self { send, token, done }
    }

    /// Stops accepting new events and waits for queued events to finish.
    #[instrument(level = "info", skip_all)]
    pub async fn close(&self) {
        //
        self.token.cancel();

        let mut done = self.done.clone();

        if let Err(error) = done.wait_for(|done| *done).await {
            tracing::error!(
                error = %error,
                "[AsyncEffectDevelop::close] background task ended without completion",
            );
        }
    }
}

impl EffectDevelop for AsyncEffectDevelop {
    #[instrument(level = "info", skip_all)]
    async fn develop<I>(&self, iter: I)
    where
        I: EventIter + Send,
    {
        if self.token.is_cancelled() {
            return;
        }

        for event in iter.into_iter() {
            if let Err(e) = self.send.try_send(event) {
                match e {
                    //
                    TrySendError::Full(_) if self.token.is_cancelled() => {
                        break;
                    }

                    //
                    TrySendError::Full(event) => {
                        tracing::warn!(
                            event = event_name(&event),
                            "[AsyncEffectDevelop::develop] event queue is full, dropping event",
                        );
                    }

                    TrySendError::Closed(_) if self.token.is_cancelled() => {
                        break;
                    }

                    TrySendError::Closed(event) => {
                        //
                        tracing::warn!(
                            event = event_name(&event),
                            "[AsyncEffectDevelop::develop] event queue is closed, dropping event",
                        );

                        break;
                    }
                }
            }
        }
    }
}

impl Drop for AsyncEffectDevelop {
    fn drop(&mut self) {
        self.token.cancel();
    }
}

/// Returns a human-readable label for a domain event variant.
fn event_name(event: &Event) -> &'static str {
    match event {
        //
        Event::UserActive(_) => "user_active",

        Event::UserSignedUp(_) => "user_signed_up",

        Event::ChapterPublished(_) => "chapter_published",

        Event::ChapterWorkflowCompleted(_) => "chapter_workflow_completed",
    }
}
