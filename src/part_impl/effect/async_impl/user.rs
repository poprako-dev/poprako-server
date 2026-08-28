//! User event actors for async side effects.

use std::borrow::Cow;
use std::collections::HashMap;

use fluent_templates::fluent_bundle::FluentValue;
use poprako_orchestra::{Context, OperRun as _};
use tracing::instrument;

use poprako_util::i18n::{trl, trl_kv};

use crate::complex::system_mail::SystemMailComplex;
use crate::model::write::system_mail::SystemMailEntry;
use crate::part::effect::event::user::{UserActiveEvent, UserSignedUpEvent};
use crate::part::repo::oper::system_mail::SendSystemMail;
use crate::part::repo::oper::team::GetTeamInfo;
use crate::part::repo::oper::user::UpdateUser;
use crate::part::repo::system_mail::SystemMailRepo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::user::UserRepo;

/// Updates the user's last-active timestamp in response to activity.
#[instrument(level = "info", skip_all)]
pub async fn touch_last_active<C, R>(repo: &R, event: UserActiveEvent)
where
    C: Context,
    R: UserRepo<C> + Sync,
{
    if (UpdateUser::TouchLastActive { id: &event.user_id })
        .run_on(repo)
        .await
        .is_err()
    {
        tracing::warn!(
            user_id = %event.user_id,
            "[AsyncEffectDevelop::touch_last_active] failed to update last-active timestamp",
        );
    }
}

/// Notifies an invitor when a user signs up through their invitation.
#[instrument(level = "info", skip_all)]
pub async fn notify_invitor<C, R>(repo: &R, event: UserSignedUpEvent)
where
    C: Context,
    R: TeamRepo<C> + SystemMailRepo + Sync,
{
    let team_info = GetTeamInfo::Id { id: &event.team_id }.run_on(repo).await;

    let Ok(team_info) = team_info else {
        //
        tracing::warn!(
            team_id = %event.team_id,
            "[AsyncEffectDevelop::notify_invitor] failed to look up team for signup notification",
        );

        return;
    };

    let mut args = HashMap::new();

    args.insert(
        Cow::Borrowed("invitee_qid"),
        FluentValue::from(event.invitee_qid.as_str()),
    );

    args.insert(
        Cow::Borrowed("team_name"),
        FluentValue::from(team_info.name.as_str()),
    );

    let system_mail_entry = SystemMailEntry {
        id: SystemMailComplex::gen_id(),
        receiver_id: event.invitor_id,
        title: trl("mail-invitation-used-title"),
        content: trl_kv("mail-invitation-used-body", &args),
    };

    if (SendSystemMail {
        entry: &system_mail_entry,
    })
    .run_on(repo)
    .await
    .is_err()
    {
        tracing::warn!(
            team_id = %event.team_id,
            receiver_id = %system_mail_entry.receiver_id,
            "[AsyncEffectDevelop::notify_invitor] failed to send signup notification",
        );
    }
}
