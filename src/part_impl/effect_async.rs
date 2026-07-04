//! Async background dispatcher for side-effect events.

use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::oneshot::{Receiver as OneshotReceiver, Sender as OneshotSender};
use tracing::{Level, instrument};

use crate::part::effect::event::Event;
use crate::part::effect::{EffectDevelop, EventIter};
use crate::part::repo::assignment::{AssignmentRepo, AssignmentRepoTransactional};
use crate::part::repo::chapter::{ChapterRepo, ChapterRepoTransactional};
use crate::part::repo::system_mail::{SystemMailRepo, SystemMailRepoTransactional};
use crate::part::repo::team::{TeamRepo, TeamRepoTransactional};
use crate::part_impl::effect_async::dispatch::dispatch;
use crate::util::DeriveTransactional;

mod chapter;
mod dispatch;
mod user;

/// Async side-effect dispatcher backed by a bounded channel.
pub struct AsyncEffectDevelop<C, R> {
    accepting: Arc<AtomicBool>,
    send: Sender<Event>,
    shutdown: Mutex<Option<OneshotSender<()>>>,
    done: Mutex<Option<OneshotReceiver<()>>>,

    _p: PhantomData<(C, R)>,
}

struct BackgroundHandler<C, R> {
    repo: Arc<R>,
    recv: Receiver<Event>,
    shutdown_recv: OneshotReceiver<()>,
    done_send: OneshotSender<()>,
    accepting: Arc<AtomicBool>,

    _p: PhantomData<C>,
}

impl<C, R> AsyncEffectDevelop<C, R>
where
    C: Send + 'static,
    R: AssignmentRepo<C> + ChapterRepo<C> + TeamRepo<C> + SystemMailRepo<C> + Send + Sync + 'static,
    <R as DeriveTransactional>::Transactional: AssignmentRepoTransactional<C>
        + ChapterRepoTransactional<C>
        + TeamRepoTransactional<C>
        + SystemMailRepoTransactional<C>,
{
    /// Creates a dispatcher and starts its background task.
    pub fn new(repo: Arc<R>, buffer_size: usize) -> Self {
        let (send, recv) = tokio::sync::mpsc::channel(buffer_size);

        let (shutdown_send, shutdown_recv) = tokio::sync::oneshot::channel();

        let (done_send, done_recv) = tokio::sync::oneshot::channel();

        let accepting = Arc::new(AtomicBool::new(true));

        let handler = BackgroundHandler {
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

impl<C, R> BackgroundHandler<C, R>
where
    C: Send,
    R: AssignmentRepo<C> + ChapterRepo<C> + TeamRepo<C> + SystemMailRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: AssignmentRepoTransactional<C>
        + ChapterRepoTransactional<C>
        + TeamRepoTransactional<C>
        + SystemMailRepoTransactional<C>,
{
    #[instrument(skip_all, level = Level::DEBUG)]
    async fn run(mut self) {
        loop {
            tokio::select! {
                event = self.recv.recv() => {
                    match event {
                        Some(event) => dispatch::<C, R>(&self.repo, event).await,
                        None => break,
                    }
                }
                _ = &mut self.shutdown_recv => {
                    self.accepting.store(false, Ordering::Release);
                    break;
                }
            }
        }

        while let Ok(event) = self.recv.try_recv() {
            dispatch::<C, R>(&self.repo, event).await;
        }

        self.done_send.send(()).unwrap_or_else(|error| {
            tracing::warn!(
                error = ?error,
                "[BackgroundHandler::run] completion receiver already dropped",
            );
        });
    }
}

#[async_trait]
impl<C, R> EffectDevelop for AsyncEffectDevelop<C, R>
where
    C: Send + Sync + 'static,
    R: AssignmentRepo<C> + ChapterRepo<C> + TeamRepo<C> + SystemMailRepo<C> + Send + Sync + 'static,
    <R as DeriveTransactional>::Transactional: AssignmentRepoTransactional<C>
        + ChapterRepoTransactional<C>
        + TeamRepoTransactional<C>
        + SystemMailRepoTransactional<C>,
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

fn event_name(event: &Event) -> &'static str {
    match event {
        Event::UserActive(_) => "user_active",
        Event::UserSignedUp(_) => "user_signed_up",
        Event::AssignmentCreated(_) => "assignment_created",
        Event::AssignmentRemoved(_) => "assignment_removed",
        Event::ChapterPublished(_) => "chapter_published",
        Event::ChapterWorkflowCompleted(_) => "chapter_workflow_completed",
        Event::ChapterWorkflowReverted(_) => "chapter_workflow_reverted",
        Event::ChapterRemoved(_) => "chapter_removed",
    }
}

#[cfg(test)]
mod tests;
