//! Event dispatcher for async side-effect handlers.

use crate::part::effect::event::Event;
use crate::part::repo::assignment::{AssignmentRepo, AssignmentRepoTransactional};
use crate::part::repo::chapter::{ChapterRepo, ChapterRepoTransactional};
use crate::part::repo::system_mail::{SystemMailRepo, SystemMailRepoTransactional};
use crate::part::repo::team::{TeamRepo, TeamRepoTransactional};
use crate::part_impl::effect_async::chapter;
use crate::part_impl::effect_async::user;
use crate::util::DeriveTransactional;

/// Dispatches a domain event to its side-effect handler.
pub async fn dispatch<C, R>(repo: &R, event: Event)
where
    R: AssignmentRepo<C> + ChapterRepo<C> + TeamRepo<C> + SystemMailRepo<C>,
    <R as DeriveTransactional>::Transactional: AssignmentRepoTransactional<C>
        + ChapterRepoTransactional<C>
        + TeamRepoTransactional<C>
        + SystemMailRepoTransactional<C>,
{
    match event {
        Event::UserActive(_) => {}
        Event::UserSignedUp(payload) => user::notify_invitor(repo, payload).await,
        Event::AssignmentCreated(_) => {}
        Event::AssignmentRemoved(_) => {}
        Event::ChapterPublished(payload) => {
            chapter::notify_reviewers_on_publish(repo, payload).await
        }
        Event::ChapterWorkflowCompleted(payload) => {
            chapter::notify_next_phase(repo, &payload).await;
            chapter::notify_reviewers_on_progress(repo, payload).await;
        }
        Event::ChapterWorkflowReverted(_) => {}
        Event::ChapterRemoved(_) => {}
    }
}
