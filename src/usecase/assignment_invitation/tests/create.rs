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

    let before = now();

    let val = create(
        (&mock, &mock, &mock),
        token("admin-user"),
        create_data("target-qid"),
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.assignment_invitations.len(), 1);

    assert_eq!(snapshot.assignment_invitations[0].id, val.id);

    assert_eq!(snapshot.assignment_invitations[0].code, val.code);

    assert_eq!(snapshot.assignment_invitations[0].chapter_id, "chapter-1");

    assert_eq!(snapshot.assignment_invitations[0].inviter_id, "admin-user");

    assert_eq!(snapshot.assignment_invitations[0].invitee_qid, "target-qid");

    assert!(snapshot.assignment_invitations[0].is_pending);

    assert_eq!(snapshot.prom_records.len(), 1);

    assert_eq!(
        snapshot.prom_records[0].payload(),
        TaskPayload::Invitation(InvitationPayload::Assignment {
            invitation_id: val.id,
        })
    );

    assert!(snapshot.prom_records[0].visible_at() >= before + EXPIRY_DELAY);

    assert!(snapshot.prom_records[0].visible_at() <= now() + EXPIRY_DELAY);
}
