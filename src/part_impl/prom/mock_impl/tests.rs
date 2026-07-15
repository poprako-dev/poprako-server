use super::*;

use crate::model::assignment_invitation::AssignmentInvitationInfo;
use crate::model::member_invitation::MemberInvitationInfo;
use crate::part::prom::payload::invitation::PurgeExpiredInvitation;
use crate::test_util::now;
use crate::value::role::{RoleField, RoleMask};

// defer_batch_records_payloads(Prom::step)(positive): batch deferral should store every record in transaction state.
// purge_expired_invitations(process_pending)(positive): pending invitations should be deleted while accepted invitations remain.

#[tokio::test]
async fn defer_batch_records_payloads() {
    //
    let mock = Mock::new();

    let prom = mock.clone();

    assert!(
        mock.coord(async move |context| {
            let ids = ["prom-1".to_string(), "prom-2".to_string()];

            let payloads = [
                Payload::Image(image::Payload::Delete {
                    object_key: "one.png".to_string(),
                }),
                Payload::Image(image::Payload::Delete {
                    object_key: "two.png".to_string(),
                }),
            ];

            let tasks = [
                Task {
                    id: &ids[0],
                    payload: &payloads[0],
                    delay: None,
                },
                Task {
                    id: &ids[1],
                    payload: &payloads[1],
                    delay: None,
                },
            ];

            prom.step(context, &DeferBatch::new(&tasks)).await?;

            Ok::<(), RegularError>(())
        })
        .await
        .is_ok()
    );

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.prom_records.len(), 2);

    assert_eq!(snapshot.prom_records[0].id(), "prom-1");

    assert_eq!(snapshot.prom_records[1].id(), "prom-2");
}

fn assignment_invitation(id: &str, pending: bool) -> AssignmentInvitationInfo {
    //
    let created_at = now();

    AssignmentInvitationInfo {
        id: id.to_string(),
        chapter_id: "chapter-1".to_string(),
        inviter_id: "user-1".to_string(),
        invitee_qid: "qid-1".to_string(),
        code: id.to_string(),
        pending,
        roles: RoleMask::from(RoleField::TRANSLATOR),
        created_at,
        updated_at: created_at,
    }
}

fn member_invitation(id: &str, pending: bool) -> MemberInvitationInfo {
    MemberInvitationInfo {
        id: id.to_string(),
        team_id: "team-1".to_string(),
        invitor: None,
        invitor_id: "user-1".to_string(),
        invitee_qid: "qid-1".to_string(),
        code: id.to_string(),
        pending,
        roles: RoleMask::from(RoleField::TRANSLATOR),
    }
}

#[tokio::test]
async fn purge_expired_invitations() {
    //
    let mock = Mock::new();

    mock.seed_assignment_invitation(assignment_invitation(
        "assignment-pending",
        true,
    ));

    mock.seed_assignment_invitation(assignment_invitation(
        "assignment-accepted",
        false,
    ));

    mock.seed_member_invitation(member_invitation("member-pending", true));

    mock.seed_member_invitation(member_invitation("member-accepted", false));

    let prom = mock.clone();

    mock.coord(async move |context| {
        //
        let ids = [
            "prom-assignment-pending".to_string(),
            "prom-assignment-accepted".to_string(),
            "prom-member-pending".to_string(),
            "prom-member-accepted".to_string(),
        ];

        let payloads = [
            Payload::PurgeExpiredInvitation(
                PurgeExpiredInvitation::Assignment {
                    invitation_id: "assignment-pending".to_string(),
                },
            ),
            Payload::PurgeExpiredInvitation(
                PurgeExpiredInvitation::Assignment {
                    invitation_id: "assignment-accepted".to_string(),
                },
            ),
            Payload::PurgeExpiredInvitation(PurgeExpiredInvitation::Member {
                invitation_id: "member-pending".to_string(),
            }),
            Payload::PurgeExpiredInvitation(PurgeExpiredInvitation::Member {
                invitation_id: "member-accepted".to_string(),
            }),
        ];

        let tasks = [
            Task {
                id: &ids[0],
                payload: &payloads[0],
                delay: None,
            },
            Task {
                id: &ids[1],
                payload: &payloads[1],
                delay: None,
            },
            Task {
                id: &ids[2],
                payload: &payloads[2],
                delay: None,
            },
            Task {
                id: &ids[3],
                payload: &payloads[3],
                delay: None,
            },
        ];

        prom.step(context, &DeferBatch::new(&tasks)).await?;

        Ok::<(), RegularError>(())
    })
    .await
    .ok()
    .unwrap();

    process_pending(&mock).await.unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.assignment_invitations.len(), 1);

    assert_eq!(snapshot.assignment_invitations[0].id, "assignment-accepted");

    assert_eq!(snapshot.member_invitations.len(), 1);

    assert_eq!(snapshot.member_invitations[0].id, "member-accepted");
}
