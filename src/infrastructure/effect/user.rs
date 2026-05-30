use crate::api::harness::HarnessInner;
use crate::domain::model::aggregate::sys_mail::SysMailCre;
use crate::domain::model::event::user::UserSignedUpEvent;
use crate::domain::query::team::TeamQeury as _;

/// Notifies the invitor via system mail that a new user has registered using
/// their invitation code.
pub async fn notify_invitor_handler(harn: &HarnessInner, event: UserSignedUpEvent) {
    // Look up the team name for the notification content.
    let team = match harn.query.get_by_id(event.team_id.clone()).await {
        Ok(team) => team,
        Err(e) => {
            tracing::error!(
                error = %e,
                team_id = %event.team_id,
                "[notify_invitor_handler] failed to look up team, skipping notification",
            );
            return;
        }
    };

    let title = "你的邀请码已被使用".to_string();
    let content = format!(
        "你的邀请码已被使用，「{}」已加入汉化组「{}」",
        event.invitee_qid, team.name,
    );

    let invitor_id = event.invitor_id;
    let mail = SysMailCre::new(invitor_id.clone(), title, content);

    if let Err(e) = harn.query.send_sys_mail(&mail).await {
        tracing::error!(
            error = %e,
            invitor_id = %invitor_id,
            invitee_qid = %event.invitee_qid,
            team_name = %team.name,
            "[notify_invitor_handler] failed to send notification mail",
        );
    }
}
