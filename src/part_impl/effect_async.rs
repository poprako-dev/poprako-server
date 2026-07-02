//! Async background dispatcher for side-effect events.

use std::borrow::Cow;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use fluent_templates::fluent_bundle::FluentValue;
use tokio::sync::mpsc::{Receiver, Sender, error::TrySendError};
use tokio::sync::oneshot::{Receiver as OneshotReceiver, Sender as OneshotSender};
use tracing::{Level, instrument};

use poprako_util::i18n::{trl, trl_kv};

use crate::complex::system_mail::SystemMailComplex;
use crate::model::system_mail::SystemMailForm;
use crate::part::effect::event::Event;
use crate::part::effect::event::user::UserSignedUpPayload;
use crate::part::effect::{EffectDevelop, EventIter};
use crate::part::repo::step::system_mail::SystemMailStep;
use crate::part::repo::step::team::TeamStep;
use crate::part::repo::system_mail::{SystemMailRepo, SystemMailRepoTransactional};
use crate::part::repo::team::{TeamRepo, TeamRepoTransactional};
use crate::part::shared::execute::Execute;
use crate::util::DeriveTransactional;

/// Async side-effect dispatcher backed by a bounded channel.
pub struct AsyncEffectDevelop<C, R> {
    accepting: Arc<AtomicBool>,
    send: Sender<Event>,
    shutdown: Mutex<Option<OneshotSender<()>>>,
    done: Mutex<Option<OneshotReceiver<()>>>,
    _context: PhantomData<C>,
    _repo: PhantomData<R>,
}

struct BackgroundHandler<C, R> {
    repo: Arc<R>,
    recv: Receiver<Event>,
    shutdown_recv: OneshotReceiver<()>,
    done_send: OneshotSender<()>,
    accepting: Arc<AtomicBool>,
    _context: PhantomData<C>,
}

impl<C, R> AsyncEffectDevelop<C, R>
where
    C: Send + 'static,
    R: TeamRepo<C> + SystemMailRepo<C> + Send + Sync + 'static,
    <R as DeriveTransactional>::Transactional:
        TeamRepoTransactional<C> + SystemMailRepoTransactional<C>,
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
            _context: PhantomData,
        };

        tokio::spawn(async move {
            handler.run().await;
        });

        Self {
            accepting,
            send,
            shutdown: Mutex::new(Some(shutdown_send)),
            done: Mutex::new(Some(done_recv)),
            _context: PhantomData,
            _repo: PhantomData,
        }
    }

    /// Stops accepting new events and waits for queued events to finish.
    pub async fn close(&self) {
        if !self.accepting.swap(false, Ordering::AcqRel) {
            return;
        }

        let shutdown_send = self.shutdown.lock().unwrap().take();

        if let Some(shutdown_send) = shutdown_send {
            let _ = shutdown_send.send(());
        }

        let done_recv = self.done.lock().unwrap().take();

        let Some(done_recv) = done_recv else {
            return;
        };

        let _ = done_recv.await;
    }
}

impl<C, R> BackgroundHandler<C, R>
where
    C: Send,
    R: TeamRepo<C> + SystemMailRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional:
        TeamRepoTransactional<C> + SystemMailRepoTransactional<C>,
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

        let _ = self.done_send.send(());
    }
}

#[async_trait]
impl<C, R> EffectDevelop for AsyncEffectDevelop<C, R>
where
    C: Send + Sync + 'static,
    R: TeamRepo<C> + SystemMailRepo<C> + Send + Sync + 'static,
    <R as DeriveTransactional>::Transactional:
        TeamRepoTransactional<C> + SystemMailRepoTransactional<C>,
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
                Ok(()) => {}
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
            }
        }
    }
}

async fn dispatch<C, R>(repo: &R, event: Event)
where
    R: TeamRepo<C> + SystemMailRepo<C>,
    <R as DeriveTransactional>::Transactional:
        TeamRepoTransactional<C> + SystemMailRepoTransactional<C>,
{
    match event {
        Event::UserActive(_) => {}
        Event::UserSignedUp(payload) => notify_invitor(repo, payload).await,
    }
}

async fn notify_invitor<C, R>(repo: &R, payload: UserSignedUpPayload)
where
    R: TeamRepo<C> + SystemMailRepo<C>,
    <R as DeriveTransactional>::Transactional:
        TeamRepoTransactional<C> + SystemMailRepoTransactional<C>,
{
    let team_info = Execute::execute(repo, &TeamStep::get_info_by_id(&payload.team_id)).await;

    let Ok(team_info) = team_info else {
        tracing::warn!(
            team_id = %payload.team_id,
            "[AsyncEffectDevelop::notify_invitor] failed to look up team for signup notification",
        );

        return;
    };

    let mut args = HashMap::new();

    args.insert(
        Cow::Borrowed("invitee_qid"),
        FluentValue::from(payload.invitee_qid.as_str()),
    );

    args.insert(
        Cow::Borrowed("team_name"),
        FluentValue::from(team_info.name.as_str()),
    );

    let system_mail_form = SystemMailForm {
        id: SystemMailComplex::gen_id(),
        receiver_id: payload.invitor_id,
        title: trl("mail-invitation-used-title"),
        content: trl_kv("mail-invitation-used-body", &args),
    };

    let result = Execute::execute(repo, &SystemMailStep::send(&system_mail_form)).await;

    if result.is_err() {
        tracing::warn!(
            team_id = %payload.team_id,
            receiver_id = %system_mail_form.receiver_id,
            "[AsyncEffectDevelop::notify_invitor] failed to send signup notification",
        );
    }
}

fn event_name(event: &Event) -> &'static str {
    match event {
        Event::UserActive(_) => "user_active",
        Event::UserSignedUp(_) => "user_signed_up",
    }
}

#[cfg(test)]
mod tests {
    // develop_dispatches_user_signup(AsyncEffectDevelop::develop)(positive): signup events should create one system mail for the invitor.
    // close_is_idempotent(AsyncEffectDevelop::close)(negative): repeated close calls should return without blocking.

    use crate::part_impl::effect_async::*;

    use crate::model::team::TeamInfo;
    use crate::part::effect::event::user::UserSignedUpPayload;
    use crate::part_impl::repo_mock::{Mock, MockContext};
    use time::OffsetDateTime;

    fn team_info() -> TeamInfo {
        let time = OffsetDateTime::now_utc();

        TeamInfo {
            id: "team-1".to_string(),
            name: "Team One".to_string(),
            description: "Team description".to_string(),
            avatar_key: None,
            avatar_uploaded: false,
            avatar_version: 0,
            workset_next_index: 0,
            created_at: time,
            updated_at: time,
        }
    }

    #[tokio::test]
    async fn develop_dispatches_user_signup() {
        let mock = Arc::new(Mock::new());

        mock.seed_team(team_info());

        let develop = AsyncEffectDevelop::<MockContext, Mock>::new(Arc::clone(&mock), 8);

        EffectDevelop::develop(
            &develop,
            Event::UserSignedUp(UserSignedUpPayload {
                team_id: "team-1".to_string(),
                invitor_id: "user-owner".to_string(),
                invitee_qid: "10001".to_string(),
            }),
        )
        .await;

        develop.close().await;

        let snapshot = mock.snapshot();

        assert_eq!(snapshot.system_mails.len(), 1);

        assert_eq!(snapshot.system_mails[0].receiver_id, "user-owner");
    }

    #[tokio::test]
    async fn close_is_idempotent() {
        let mock = Arc::new(Mock::new());

        let develop = AsyncEffectDevelop::<MockContext, Mock>::new(mock, 8);

        develop.close().await;

        develop.close().await;
    }
}
