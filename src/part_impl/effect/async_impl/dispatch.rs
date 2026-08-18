//! Event dispatcher for async side-effect handlers.

use poprako_orchestra::Context;
use tracing::instrument;

use crate::part::effect::event::Event;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::system_mail::SystemMailRepo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::user::UserRepo;
use crate::part_impl::effect::async_impl::{chapter, user};

/// Dispatches a domain event to its side-effect handler.
#[instrument(level = "info", skip_all)]
pub async fn dispatch<C, R>(repo: &R, event: Event)
where
    C: Context,
    R: AssignmentRepo<C>
        + ChapterRepo<C>
        + TeamRepo<C>
        + SystemMailRepo
        + UserRepo<C>,
{
    match event {
        //
        Event::UserActive { payload } => {
            user::touch_last_active(repo, payload).await
        }

        Event::UserSignedUp { payload } => {
            user::notify_invitor(repo, payload).await
        }

        Event::ChapterPublished { payload } => {
            chapter::notify_reviewers_on_publish(repo, payload).await
        }

        Event::ChapterWorkflowCompleted { payload } => {
            //
            chapter::notify_next_phase(repo, &payload).await;

            chapter::notify_reviewers_on_progress(repo, payload).await;
        }
    }
}
