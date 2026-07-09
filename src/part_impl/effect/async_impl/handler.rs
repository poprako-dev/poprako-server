//! Background event handler — receives events from the channel and
//! dispatches them to the appropriate domain handler.

use std::marker::PhantomData;
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

pub(super) struct EffectHandler<C, R> {
    pub(super) repo: Arc<R>,
    pub(super) recv: Receiver<Event>,
    pub(super) shutdown_recv: OneshotReceiver<()>,
    pub(super) done_send: OneshotSender<()>,
    pub(super) accepting: Arc<AtomicBool>,

    pub(super) _p: PhantomData<C>,
}

impl<C, R> EffectHandler<C, R>
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
    #[instrument(skip_all, level = Level::DEBUG)]
    pub(super) async fn run(mut self) {
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
