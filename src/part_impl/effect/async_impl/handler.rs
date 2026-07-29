//! Background event handler — receives events from the channel and
//! dispatches them to the appropriate domain handler.

use std::sync::Arc;

use tokio::sync::mpsc::Receiver;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use crate::part::effect::event::Event;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::system_mail::SystemMailRepo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::user::UserRepo;
use crate::part_impl::effect::async_impl::dispatch::dispatch;

/// Background event consumer that receives events from the channel and
/// dispatches them to the appropriate domain handler.
pub struct EffectHandler<R> {
    repo: Arc<R>,
    recv: Receiver<Event>,
    token: CancellationToken,
}

impl<R> EffectHandler<R> {
    /// Builds a background handler from its queue and cancellation token.
    pub fn new(
        repo: Arc<R>,
        recv: Receiver<Event>,
        token: CancellationToken,
    ) -> Self {
        Self { repo, recv, token }
    }

    #[instrument(level = "info", skip_all)]
    /// Runs the event consumer loop, dispatching events until a shutdown signal is received.
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
                () = self.token.cancelled() => break,
            }
        }

        while let Ok(event) = self.recv.try_recv() {
            dispatch::<C, R>(&self.repo, event).await;
        }
    }
}
