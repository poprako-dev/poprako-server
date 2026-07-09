//! User event handlers for async side effects.

use std::borrow::Cow;
use std::collections::HashMap;

use fluent_templates::fluent_bundle::FluentValue;

use poprako_util::i18n::{trl, trl_kv};

use crate::complex::system_mail::SystemMailComplex;
use crate::model::system_mail::SystemMailForm;
use crate::part::effect::event::user::{
    UserActivePayload, UserSignedUpPayload,
};
use crate::part::repo::step::system_mail::SystemMailStep;
use crate::part::repo::step::team::TeamStep;
use crate::part::repo::step::user::UserStep;
use crate::part::repo::system_mail::{
    SystemMailRepo, SystemMailRepoTransactional,
};
use crate::part::repo::team::{TeamRepo, TeamRepoTransactional};
use crate::part::repo::user::{UserRepo, UserRepoTransactional};
use crate::part::shared::execute::Execute;
use crate::util::DeriveTransactional;

/// Updates the user's last-active timestamp in response to activity.
pub async fn touch_last_active<C, R>(repo: &R, payload: UserActivePayload)
where
    R: UserRepo<C>,
    <R as DeriveTransactional>::Transactional: UserRepoTransactional<C>,
{
    let result =
        Execute::execute(repo, &UserStep::touch_last_active(&payload.user_id))
            .await;

    if result.is_err() {
        tracing::warn!(
            user_id = %payload.user_id,
            "[AsyncEffectDevelop::touch_last_active] failed to update last-active timestamp",
        );
    }
}

/// Notifies an invitor when a user signs up through their invitation.
pub async fn notify_invitor<C, R>(repo: &R, payload: UserSignedUpPayload)
where
    R: TeamRepo<C> + SystemMailRepo<C>,
    <R as DeriveTransactional>::Transactional:
        TeamRepoTransactional<C> + SystemMailRepoTransactional<C>,
{
    let team_info =
        Execute::execute(repo, &TeamStep::get_info_by_id(&payload.team_id))
            .await;

    let Ok(team_info) = team_info else {
        //
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

    let result =
        Execute::execute(repo, &SystemMailStep::send(&system_mail_form)).await;

    if result.is_err() {
        tracing::warn!(
            team_id = %payload.team_id,
            receiver_id = %system_mail_form.receiver_id,
            "[AsyncEffectDevelop::notify_invitor] failed to send signup notification",
        );
    }
}
