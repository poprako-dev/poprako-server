use super::*;

use crate::model::read::proj::assignment_invitation::AssignmentInvitationInfo;
use crate::model::read::proj::member_invitation::MemberInvitationInfo;
use crate::model::read::proj::page::PageInfo;
use crate::part::prom::payload::invitation::InvitationPayload;
use crate::test_util::now;
use crate::value::image::{ImageExt, ImageHash};
use crate::value::role::{RoleField, RoleMask};

// defer_batch_records_payloads(Prom::step)(positive): batch deferral should store every record in transaction state.
// purge_expired_invitations(process_pending)(positive): pending invitations should be deleted while accepted invitations remain.

// defer_records_payload(Prom::step)(positive): individual deferral should store the record in transaction state.
// defer_batch_records_payloads(Prom::step)(positive): batch deferral should store every record in transaction state.
// process_pending_marks_uploaded_image(process_pending)(positive): check-upload should mark matching uploaded image state.
// process_pending_ignores_stale_image_check(process_pending)(positive): stale check-upload should not mutate the current resource or delete its old object.
// process_pending_rejects_mismatched_image_key(process_pending)(negative): current-version check-upload should require the persisted object key.
// process_pending_deletes_missing_resource_image(process_pending)(positive): check-upload should delete an object when the owning resource disappears.

// Build a deterministic user payload for deferred-image assertions.
fn user_info(id: &str, avatar_key: &str, avatar_version: u32) -> UserInfo {
    //
    // Keep all timestamps in one place so the tests can reuse deterministic users.
    let now = OffsetDateTime::now_utc();

    UserInfo {
        id: id.to_string(),
        qid: format!("qid-{}", id),
        nickname: format!("nick-{}", id),
        avatar_key: Some(avatar_key.to_string()),
        is_avatar_uploaded: Some(false),
        avatar_version: Some(avatar_version),
        avatar_hash: Some(ImageHash::default()),
        avatar_ext: Some(ImageExt::Png),
        is_sadmin: false,
        last_active_at: now,
        created_at: now,
        updated_at: now,
    }
}

// Build deterministic credentials that match a user id in tests.
fn user_credential(id: &str) -> UserCredential {
    UserCredential {
        user_id: id.to_string(),
        password_hash: format!("hash-{}", id),
    }
}

fn cleared_page_info(id: &str) -> PageInfo {
    let created_at = now();

    PageInfo {
        id: id.to_string(),
        chapter_id: "chapter-1".to_string(),
        index: 0,
        image_key: None,
        is_image_uploaded: None,
        image_version: None,
        image_hash: None,
        image_ext: None,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        created_at,
        updated_at: created_at,
    }
}

// Internal implementation of `defer_payload`.
async fn defer_payload(
    mock: &Mock,
    context: &mut MockContext,
    id: &str,
    payload: TaskPayload,
) -> BaseRest<()> {
    //
    // Internal implementation detail.
    let id = id.to_string();

    let task = Task {
        id: &id,
        payload: &payload,
        delay: None,
    };

    mock.step(context, &Defer::new(task)).await
}

// Internal implementation of `assignment_invitation`.
fn assignment_invitation(id: &str, pending: bool) -> AssignmentInvitationInfo {
    //
    // Internal implementation detail.
    let created_at = now();

    AssignmentInvitationInfo {
        id: id.to_string(),
        chapter_id: "chapter-1".to_string(),
        inviter_id: "user-1".to_string(),
        invitee_qid: "qid-1".to_string(),
        code: id.to_string(),
        is_pending: pending,
        roles: RoleMask::from(RoleField::TRANSLATOR),
        created_at,
        updated_at: created_at,
    }
}

// Internal implementation of `member_invitation`.
fn member_invitation(id: &str, pending: bool) -> MemberInvitationInfo {
    MemberInvitationInfo {
        id: id.to_string(),
        team_id: "team-1".to_string(),
        invitor: None,
        invitor_id: "user-1".to_string(),
        invitee_qid: "qid-1".to_string(),
        code: id.to_string(),
        is_pending: pending,
        roles: RoleMask::from(RoleField::TRANSLATOR),
    }
}

#[tokio::test]
async fn defer_records_payload() {
    //
    // Internal implementation detail.
    let mock = Mock::new();

    let before = OffsetDateTime::now_utc();

    let prom = mock.clone();

    assert!(
        mock.coord(async move |context| {
            defer_payload(
                &prom,
                context,
                "prom-1",
                TaskPayload::Image(image::ImagePayload::Delete {
                    object_key: "key".to_string(),
                }),
            )
            .await?;

            Ok::<(), BaseError>(())
        })
        .await
        .is_ok()
    );

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.prom_records.len(), 1);

    assert_eq!(snapshot.prom_records[0].id(), "prom-1");

    assert!(snapshot.prom_records[0].visible_at() >= before);
}

#[tokio::test]
async fn process_pending_marks_uploaded_image() {
    //
    // Internal implementation detail.
    let mock = Mock::new();

    mock.seed_user(
        user_info("user-1", "avatar.png", 1),
        user_credential("user-1"),
    );

    let prom = mock.clone();

    mock.coord(async move |context| {
        //
        // Internal implementation detail.
        defer_payload(
            &prom,
            context,
            "prom-1",
            TaskPayload::Image(image::ImagePayload::CheckUpload {
                resource_kind: image::ResourceKind::UserAvatar,
                resource_id: "user-1".to_string(),
                object_key: "avatar.png".to_string(),
                version: 1,
            }),
        )
        .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .ok()
    .unwrap();

    process_pending(&mock).await.ok().unwrap();

    assert_eq!(mock.snapshot().users[0].is_avatar_uploaded, Some(true));
}

#[tokio::test]
async fn process_pending_ignores_stale_image_check() {
    //
    // Internal implementation detail.
    let mock = Mock::new();

    mock.seed_user(
        user_info("user-1", "avatar-v2.png", 2),
        user_credential("user-1"),
    );

    let prom = mock.clone();

    mock.coord(async move |context| {
        //
        // Internal implementation detail.
        defer_payload(
            &prom,
            context,
            "prom-1",
            TaskPayload::Image(image::ImagePayload::CheckUpload {
                resource_kind: image::ResourceKind::UserAvatar,
                resource_id: "user-1".to_string(),
                object_key: "avatar-v1.png".to_string(),
                version: 1,
            }),
        )
        .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .ok()
    .unwrap();

    process_pending(&mock).await.ok().unwrap();

    let snapshot = mock.snapshot();

    assert_ne!(snapshot.users[0].is_avatar_uploaded, Some(true));

    assert!(snapshot.deleted_image_keys.is_empty());
}

#[tokio::test]
async fn process_pending_does_not_revive_cleared_page_image() {
    let mock = Mock::new();

    mock.seed_page(cleared_page_info("page-1"));

    let prom = mock.clone();

    mock.coord(async move |context| {
        defer_payload(
            &prom,
            context,
            "prom-1",
            TaskPayload::Image(image::ImagePayload::CheckUpload {
                resource_kind: image::ResourceKind::PageImage,
                resource_id: "page-1".to_string(),
                object_key: "page-1.png".to_string(),
                version: 1,
            }),
        )
        .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .unwrap();

    process_pending(&mock).await.unwrap();

    let page_info = &mock.snapshot().pages[0];

    assert_eq!(page_info.image_key, None);

    assert_eq!(page_info.is_image_uploaded, None);

    assert_eq!(page_info.image_version, None);

    assert_eq!(page_info.image_hash, None);

    assert_eq!(page_info.image_ext, None);
}

#[tokio::test]
async fn process_pending_rejects_mismatched_image_key() {
    //
    // Internal implementation detail.
    let mock = Mock::new();

    mock.seed_user(
        user_info("user-1", "avatar-current.png", 1),
        user_credential("user-1"),
    );

    let prom = mock.clone();

    mock.coord(async move |context| {
        //
        // Internal implementation detail.
        defer_payload(
            &prom,
            context,
            "prom-1",
            TaskPayload::Image(image::ImagePayload::CheckUpload {
                resource_kind: image::ResourceKind::UserAvatar,
                resource_id: "user-1".to_string(),
                object_key: "avatar-other.png".to_string(),
                version: 1,
            }),
        )
        .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .ok()
    .unwrap();

    assert!(process_pending(&mock).await.is_err());

    assert_ne!(mock.snapshot().users[0].is_avatar_uploaded, Some(true));
}

#[tokio::test]
async fn process_pending_keeps_missing_resource_image() {
    //
    // Internal implementation detail.
    let mock = Mock::new();

    let prom = mock.clone();

    mock.coord(async move |context| {
        //
        // Internal implementation detail.
        defer_payload(
            &prom,
            context,
            "prom-1",
            TaskPayload::Image(image::ImagePayload::CheckUpload {
                resource_kind: image::ResourceKind::UserAvatar,
                resource_id: "missing-user".to_string(),
                object_key: "orphan-avatar.png".to_string(),
                version: 1,
            }),
        )
        .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .ok()
    .unwrap();

    process_pending(&mock).await.ok().unwrap();

    assert!(mock.snapshot().deleted_image_keys.is_empty());
}

#[tokio::test]
async fn defer_batch_records_payloads() {
    //
    // Internal implementation detail.
    let mock = Mock::new();

    let prom = mock.clone();

    assert!(
        mock.coord(async move |context| {
            let ids = ["prom-1".to_string(), "prom-2".to_string()];

            let payloads = [
                TaskPayload::Image(image::ImagePayload::Delete {
                    object_key: "one.png".to_string(),
                }),
                TaskPayload::Image(image::ImagePayload::Delete {
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

            Ok::<(), BaseError>(())
        })
        .await
        .is_ok()
    );

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.prom_records.len(), 2);

    assert_eq!(snapshot.prom_records[0].id(), "prom-1");

    assert_eq!(snapshot.prom_records[1].id(), "prom-2");
}

#[tokio::test]
async fn purge_expired_invitations() {
    //
    // Internal implementation detail.
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
        // Internal implementation detail.
        let ids = [
            "prom-assignment-pending".to_string(),
            "prom-assignment-accepted".to_string(),
            "prom-member-pending".to_string(),
            "prom-member-accepted".to_string(),
        ];

        let payloads = [
            TaskPayload::Invitation(InvitationPayload::Assignment {
                invitation_id: "assignment-pending".to_string(),
            }),
            TaskPayload::Invitation(InvitationPayload::Assignment {
                invitation_id: "assignment-accepted".to_string(),
            }),
            TaskPayload::Invitation(InvitationPayload::Member {
                invitation_id: "member-pending".to_string(),
            }),
            TaskPayload::Invitation(InvitationPayload::Member {
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

        Ok::<(), BaseError>(())
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
