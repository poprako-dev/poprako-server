//! Background event handler — receives events from the channel and
//! dispatches them to the appropriate domain handler.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::mpsc::Receiver;
use tokio::sync::oneshot::{
    Receiver as OneshotReceiver, Sender as OneshotSender,
};
use tracing::{Level, instrument};

use crate::part::effect::event::Event;
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

use crate::part_impl::effect::async_impl::dispatch::dispatch;

/// Background event consumer that receives events from the channel and
/// dispatches them to the appropriate domain handler.
pub struct EffectHandler<R> {
    repo: Arc<R>,
    recv: Receiver<Event>,
    shutdown_recv: OneshotReceiver<()>,
    done_send: OneshotSender<()>,
    accepting: Arc<AtomicBool>,
}

impl<R> EffectHandler<R> {
    /// Builds a background handler from its queue and shutdown channels.
    pub fn new(
        repo: Arc<R>,
        recv: Receiver<Event>,
        shutdown_recv: OneshotReceiver<()>,
        done_send: OneshotSender<()>,
        accepting: Arc<AtomicBool>,
    ) -> Self {
        Self {
            repo,
            recv,
            shutdown_recv,
            done_send,
            accepting,
        }
    }

    #[instrument(skip_all, level = Level::DEBUG)]
    pub async fn run<C>(mut self)
    where
        C: Send,
        R: AssignmentRepo<C>
            + ChapterRepo<C>
            + TeamRepo<C>
            + SystemMailRepo<C>
            + UserRepo<C>
            + Send
            + Sync,
        <R as DeriveTransactional>::Transactional: AssignmentRepoTransactional<C>
            + ChapterRepoTransactional<C>
            + TeamRepoTransactional<C>
            + SystemMailRepoTransactional<C>
            + UserRepoTransactional<C>,
    {
        loop {
            tokio::select! {
                event = self.recv.recv() => {
                    match event {
                        Some(event) => {
                            dispatch::<C, R>(&self.repo, event).await
                        }
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
                "[EffectHandler::run] completion receiver already dropped",
            );
        });
    }
}
