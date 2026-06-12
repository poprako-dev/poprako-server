use std::borrow::Cow;
use std::collections::HashMap;

use fluent_templates::fluent_bundle::FluentValue;
use tracing::{instrument, Level};

use poprako_util::i18n::{trl, trl_kv};

use crate::domain::model::aggr::system_mail::{SystemMailAggr, SystemMailForm};
use crate::domain::model::event::user::UserSignedUpEvent;
use crate::domain::query::system_mail::SystemMailQuery;
use crate::domain::query::team::TeamQuery;

/// Notifies the invitor via system mail that a new user has registered using
/// their invitation code.
#[instrument(skip(harn, event), level = Level::DEBUG)]
pub async fn notify_invitor<H>(harn: &H, event: UserSignedUpEvent)
where
    H: TeamQuery + SystemMailQuery,
{
    // Look up the team name for the notification content.
    let Some(team) = TeamQuery::get_by_id(harn, &event.team_id).await.ok() else {
        tracing::error!(
            team_id = %event.team_id,
            "[notify_invitor_handler] failed to look up team for notification",
        );
        return;
    };

    let invitor_id = event.invitor_id;
    let title = trl("error-invitation-used-title");
    let content = trl_kv(
        "error-invitation-used-body",
        &HashMap::from([
            (
                Cow::Borrowed("invitee_qid"),
                FluentValue::from(event.invitee_qid.as_str()),
            ),
            (
                Cow::Borrowed("team_name"),
                FluentValue::from(team.name.as_str()),
            ),
        ]),
    );

    let mail = SystemMailForm {
        id: SystemMailAggr::generate_id(),
        receiver_id: invitor_id,
        title,
        content,
    };

    let _ = SystemMailQuery::send(harn, &mail).await;
}
