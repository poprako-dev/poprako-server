//! Async background dispatcher for side-effect events.

use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::oneshot::{
    Receiver as OneshotReceiver, Sender as OneshotSender,
};

use crate::part::effect::event::Event;
use crate::part::effect::{EffectDevelop, EventIter};
use crate::part::repo::assignment::{
    AssignmentRepo, AssignmentRepoTransactional,
};
use crate::part::repo::chapter::{ChapterRepo, ChapterRepoTransactional};
use crate::part::repo::system_mail::{
    SystemMailRepo, SystemMailRepoTransactional,
};
use crate::part::repo::team::{TeamRepo, TeamRepoTransactional};
use crate::part::repo::user::{UserRepo, UserRepoTransactional};
use crate::util::DeriveTransactional;

/// Chapter event handlers.
mod chapter;
/// Event dispatch logic.
mod dispatch;
/// Background event handler runner.
mod handler;
/// User event handlers.
mod user;

/// Async side-effect dispatcher backed by a bounded channel.
pub struct AsyncEffectDevelop<C, R> {
    accepting: Arc<AtomicBool>,
    send: Sender<Event>,
    shutdown: Mutex<Option<OneshotSender<()>>>,
    done: Mutex<Option<OneshotReceiver<()>>>,

    _p: PhantomData<(C, R)>,
}

impl<C, R> AsyncEffectDevelop<C, R>
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
    <R as DeriveTransactional>::Transactional: AssignmentRepoTransactional<C>
        + ChapterRepoTransactional<C>
        + TeamRepoTransactional<C>
        + SystemMailRepoTransactional<C>
        + UserRepoTransactional<C>,
{
    /// Creates a dispatcher and starts its background task.
    pub fn new(repo: Arc<R>, buffer_size: usize) -> Self {
        let (send, recv) = tokio::sync::mpsc::channel(buffer_size);

        let (shutdown_send, shutdown_recv) = tokio::sync::oneshot::channel();

        let (done_send, done_recv) = tokio::sync::oneshot::channel();

        let accepting = Arc::new(AtomicBool::new(true));

        let handler = handler::EffectHandler {
            repo,
            recv,
            shutdown_recv,
            done_send,
            accepting: Arc::clone(&accepting),
            _p: PhantomData,
        };

        tokio::spawn(async move {
            handler.run().await;
        });

        Self {
            accepting,
            send,
            shutdown: Mutex::new(Some(shutdown_send)),
            done: Mutex::new(Some(done_recv)),
            _p: PhantomData,
        }
    }

    /// Stops accepting new events and waits for queued events to finish.
    pub async fn close(&self) {
        if !self.accepting.swap(false, Ordering::AcqRel) {
            return;
        }

        let shutdown_send = self.shutdown.lock().unwrap().take();

        if let Some(shutdown_send) = shutdown_send {
            shutdown_send.send(()).unwrap_or_else(|error| {
                tracing::error!(
                    error = ?error,
                    "[AsyncEffectDevelop::close] background task already terminated",
                );
            });
        }

        let done_recv = self.done.lock().unwrap().take();

        let Some(done_recv) = done_recv else {
            return;
        };

        done_recv.await.unwrap_or_else(|error| {
            tracing::error!(
                error = %error,
                "[AsyncEffectDevelop::close] background task did not signal completion",
            );
        });
    }
}

#[async_trait]
impl<C, R> EffectDevelop for AsyncEffectDevelop<C, R>
where
    C: Send + Sync + 'static,
    R: AssignmentRepo<C>
        + ChapterRepo<C>
        + TeamRepo<C>
        + SystemMailRepo<C>
        + UserRepo<C>
        + Send
        + Sync
        + 'static,
    <R as DeriveTransactional>::Transactional: AssignmentRepoTransactional<C>
        + ChapterRepoTransactional<C>
        + TeamRepoTransactional<C>
        + SystemMailRepoTransactional<C>
        + UserRepoTransactional<C>,
{
    async fn develop<I>(&self, iter: I)
    where
        I: EventIter + Send,
    {
        if !self.accepting.load(Ordering::Acquire) {
            return;
        }

        for event in iter.into_iter() {
            match self.send.try_send(event) {
                Err(TrySendError::Full(event)) => {
                    tracing::warn!(
                        event = event_name(&event),
                        "[AsyncEffectDevelop::develop] event queue is full, dropping event",
                    );
                }
                Err(TrySendError::Closed(event)) => {
                    tracing::warn!(
                        event = event_name(&event),
                        "[AsyncEffectDevelop::develop] event queue is closed, dropping event",
                    );
                    break;
                }
                _ => {}
            }
        }
    }
}

/// Returns a human-readable label for a domain event variant.
fn event_name(event: &Event) -> &'static str {
    match event {
        Event::UserActive(_) => "user_active",
        Event::UserSignedUp(_) => "user_signed_up",
        Event::ChapterPublished(_) => "chapter_published",
        Event::ChapterWorkflowCompleted(_) => "chapter_workflow_completed",
    }
}

#[cfg(test)]
mod tests;
