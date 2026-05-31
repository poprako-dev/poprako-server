use std::ops::Deref;

use crate::api::harness::HarnessBase;
use crate::domain::model::aggregate::system_mail::SystemMailForm;
use crate::domain::model::event::user::UserSignedUpEvent;
use crate::domain::query::system_mail::SystemMailQuery;
use crate::domain::query::team::TeamQuery;

/// Notifies the invitor via system mail that a new user has registered using
/// their invitation code.
pub async fn notify_invitor_handler(harn: &HarnessBase, event: UserSignedUpEvent) {
    // Look up the team name for the notification content.
    let Some(team) = TeamQuery::get_by_id(harn.deref(), &event.team_id)
        .await
        .ok()
    else {
        tracing::error!(
            team_id = %event.team_id,
            "[notify_invitor_handler] failed to look up team for notification",
        );
        return;
    };

    let title = "你的邀请码已被使用".to_string();
    let content = format!(
        "你的邀请码已被使用，「{}」已加入汉化组「{}」",
        event.invitee_qid, team.name,
    );

    let invitor_id = event.invitor_id;

    let mail = SystemMailForm::new(invitor_id.clone(), title, content);

    if let Err(e) = SystemMailQuery::send(harn.deref(), &mail).await {
        tracing::error!(
            error = %e,
            invitor_id = %invitor_id,
            invitee_qid = %event.invitee_qid,
            team_name = %team.name,
            "[notify_invitor_handler] failed to send notification mail",
        );
    }
}
