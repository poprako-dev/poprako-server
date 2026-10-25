//! Assignment invitation creation tests.

use super::*;

#[tokio::test]
async fn create_reviewer_creates_pending_invitation() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    seed_admin(&mock);

    mock.seed_user(
        user("target-user", "target-qid", "Target"),
        credential("target-user"),
    );

    let config = InvitationConfig {
        assignment_ttl: std::time::Duration::from_secs(37),
        ..InvitationConfig::default()
    };

    let before = now();

    let val = create(
        (&mock, &mock, &mock, &config),
        token("admin-user"),
        create_data("\u{2003}target-qid\u{00a0}"),
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.assignment_invitations.len(), 1);

    assert_eq!(snapshot.assignment_invitations.first().unwrap().id, val.id);

    assert_eq!(
        snapshot.assignment_invitations.first().unwrap().code,
        val.code
    );

    assert_eq!(
        snapshot.assignment_invitations.first().unwrap().chapter_id,
        "chapter--00000000001"
    );

    assert_eq!(
        snapshot.assignment_invitations.first().unwrap().inviter_id,
        "admin-user"
    );

    assert_eq!(
        snapshot.assignment_invitations.first().unwrap().invitee_qid,
        "target-qid"
    );

    assert!(snapshot.assignment_invitations.first().unwrap().is_pending);

    assert_eq!(snapshot.prom_records.len(), 1);

    assert_eq!(
        snapshot.prom_records.first().unwrap().payload(),
        PromPayload::Invitation {
            payload: InvitationPayload::PurgeExpiredAssignmentInvitation {
                invitation_id: val.id,
            },
        }
    );

    assert!(
        snapshot.prom_records.first().unwrap().visible_at()
            >= before + config.assignment_ttl
    );

    assert!(
        snapshot.prom_records.first().unwrap().visible_at()
            <= now() + config.assignment_ttl
    );
}
