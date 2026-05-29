use crate::api::harness::HarnessInner;
use crate::domain::model::event::user::UserSignedUpEvent;

/// Notifies the invitor that their invitation code has been used by a new
/// registrant.
pub async fn notify_invitor_handler(_harn: &HarnessInner, event: UserSignedUpEvent) {
    tracing::info!(
        team_id = %event.team_id,
        invitor_id = %event.invitor_id,
        invitee_qid = %event.invitor_qid,
        "[notify_invitor_handler] invitation code used",
    );

    // TODO: implement actual notification (sys_mail) using harn repos
}
