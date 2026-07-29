// archive(archive)(positive): archive should retain immutable payloads, queue every image key, and remove active descendants.
// archive(archive)(negative): non-admin callers should not create archive rows or delete active data.
// archive(archive)(negative): archive persistence failure should roll back payload, outbox, and active-data changes.
// export(export)(positive): retained month slots should return stored JSON strings without decoding.

use super::*;

use time::OffsetDateTime;

use crate::model::assignment::AssignmentInfo;
use crate::model::assignment_invitation::AssignmentInvitationInfo;
use crate::model::chapter::ChapterInfo;
use crate::model::comic::ComicInfo;
use crate::model::member::MemberInfo;
use crate::model::page::PageInfo;
use crate::model::unit::UnitInfo;
use crate::model::user::{UserCredential, UserInfo, UserToken};
use crate::model::workset::WorksetInfo;
use crate::part::prom::payload::Payload;
use crate::part::prom::payload::image::Payload as ImagePayload;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;
use crate::value::chapter::StageMask;
use crate::value::image::{ImageExtension, ImageHash};
use crate::value::role::{RoleField, RoleMask};

fn seed_archive_scope(mock: &Mock, member_roles: RoleMask) {
    //
    let archived_at = OffsetDateTime::now_utc();

    let stage_mask = StageMask::try_from(0).unwrap();

    mock.seed_user(
        UserInfo {
            id: "user-1".into(),
            qid: "qid-user-1".into(),
            nickname: "archiver".into(),
            avatar_key: Some("avatars/user-1.png".into()),
            avatar_uploaded: true,
            avatar_version: 3,
            is_sadmin: false,
            last_active_at: archived_at,
            created_at: archived_at,
            updated_at: archived_at,
        },
        UserCredential {
            user_id: "user-1".into(),
            password_hash: "hashed".into(),
        },
    );

    mock.seed_workset(WorksetInfo {
        id: "workset-1".into(),
        team_id: "team-1".into(),
        index: 4,
        name: "workset".into(),
        description: Some("archive scope".into()),
        comic_count: 7,
        created_at: archived_at,
        updated_at: archived_at,
    });

    mock.seed_member(MemberInfo {
        id: "member-1".into(),
        user_id: "user-1".into(),
        user_nickname: "archiver".into(),
        user_last_active_at: archived_at,
        team_id: "team-1".into(),
        user: None,
        team: None,
        roles: member_roles,
    });

    mock.seed_comic(ComicInfo {
        id: "comic-1".into(),
        workset_id: "workset-1".into(),
        index: 2,
        title: "comic title".into(),
        author: "comic author".into(),
        description: Some("comic description".into()),
        cover_key: Some("covers/reserved.png".into()),
        cover_uploaded: false,
        cover_version: 5,
        chapter_count: 1,
        creator_id: "user-1".into(),
        workset: None,
        team: None,
        creator: None,
        last_active_at: archived_at,
        created_at: archived_at,
        updated_at: archived_at,
    });

    mock.seed_chapter(ChapterInfo {
        id: "chapter-1".into(),
        comic_id: "comic-1".into(),
        comic: None,
        is_pinned: true,
        index: 0,
        subtitle: "chapter subtitle".into(),
        page_count: 1,
        total_unit_count: 1,
        translated_unit_count: 1,
        proofread_unit_count: 1,
        stages: stage_mask,
        creator_id: "user-1".into(),
        creator: None,
        created_at: archived_at,
        updated_at: archived_at,
    });

    mock.seed_assignment(AssignmentInfo {
        id: "assignment-1".into(),
        chapter_id: "chapter-1".into(),
        user_id: "user-1".into(),
        user: None,
        chapter: None,
        roles: RoleMask::from(RoleField::TRANSLATOR),
        created_at: archived_at,
        updated_at: archived_at,
    });

    mock.seed_assignment_invitation(AssignmentInvitationInfo {
        id: "invitation-1".into(),
        chapter_id: "chapter-1".into(),
        inviter_id: "user-1".into(),
        invitee_qid: "qid-invitee".into(),
        code: "invite-code".into(),
        pending: true,
        roles: RoleMask::from(RoleField::PROOFREADER),
        created_at: archived_at,
        updated_at: archived_at,
    });

    mock.seed_page(PageInfo {
        id: "page-1".into(),
        chapter_id: "chapter-1".into(),
        index: 0,
        image_key: Some("pages/reserved.png".into()),
        image_uploaded: false,
        image_version: 4,
        image_hash: ImageHash::new([0u8; 32]),
        image_byte_length: 4096,
        image_extension: ImageExtension::Webp,
        total_unit_count: 1,
        translated_unit_count: 1,
        proofread_unit_count: 1,
        created_at: archived_at,
        updated_at: archived_at,
    });

    mock.seed_unit(UnitInfo {
        id: "unit-1".into(),
        page_id: "page-1".into(),
        index: 0,
        is_bubble: true,
        is_proofread: true,
        x_coord: 1.5,
        y_coord: 2.5,
        translated_text: Some("translated".into()),
        last_translator_id: Some("user-1".into()),
        proofread_text: Some("proofread".into()),
        last_proofreader_id: Some("user-1".into()),
        created_at: archived_at,
        updated_at: archived_at,
    });
}

fn token() -> UserToken {
    UserToken {
        user_id: "user-1".into(),
    }
}

#[tokio::test]
async fn archive_retains_payloads_queues_images_and_deletes_active_data() {
    //
    let mock = Mock::new();

    seed_archive_scope(&mock, RoleMask::from(RoleField::ADMIN));

    let archive_comic_val =
        archive(&mock, &mock, &mock, token(), "comic-1".into())
            .await
            .unwrap();

    let snapshot = mock.snapshot();

    assert_ne!(archive_comic_val.archived_comic_id, "comic-1");

    assert!(snapshot.comics.is_empty());

    assert!(snapshot.chapters.is_empty());

    assert!(snapshot.assignments.is_empty());

    assert!(snapshot.assignment_invitations.is_empty());

    assert!(snapshot.pages.is_empty());

    assert!(snapshot.units.is_empty());

    assert_eq!(snapshot.worksets[0].comic_count, 7);

    assert_eq!(snapshot.comic_archives.len(), 1);

    assert_eq!(snapshot.comic_archives[0].team_id, "team-1");

    assert_eq!(snapshot.comic_archives[0].archiver_id, "user-1");

    let archived_comic_payload: serde_json::Value =
        serde_json::from_str(&snapshot.comic_archives[0].archived_payload)
            .unwrap();

    assert_eq!(archived_comic_payload["source_comic_id"], "comic-1");

    assert_eq!(archived_comic_payload["workset"]["id"], "workset-1");

    assert_eq!(
        archived_comic_payload["chapters"].as_array().unwrap().len(),
        1
    );

    assert_eq!(
        archived_comic_payload["chapters"][0]["source_chapter_id"],
        "chapter-1"
    );

    assert_eq!(
        archived_comic_payload["chapters"][0]["assignments"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    assert_eq!(
        archived_comic_payload["chapters"][0]["assignments"][0]["user"]["nickname"],
        "archiver"
    );

    assert_eq!(
        archived_comic_payload["chapters"][0]["pages"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    assert_eq!(
        archived_comic_payload["chapters"][0]["pages"][0]["source_page_id"],
        "page-1"
    );

    assert_eq!(
        archived_comic_payload["chapters"][0]["pages"][0]["units"][0]["source_unit_id"],
        "unit-1"
    );

    let mut deleted_image_keys = snapshot
        .prom_records
        .iter()
        .filter_map(|prom_record| match prom_record.payload() {
            //
            Payload::Image(ImagePayload::Delete { object_key }) => {
                Some(object_key.to_string())
            }

            _ => None,
        })
        .collect::<Vec<_>>();

    deleted_image_keys.sort();

    assert_eq!(
        deleted_image_keys,
        vec!["covers/reserved.png", "pages/reserved.png"]
    );
}

#[tokio::test]
async fn export_returns_stored_strings_grouped_by_month() {
    let mock = Mock::new();

    seed_archive_scope(&mock, RoleMask::from(RoleField::ADMIN));

    archive(&mock, &mock, &mock, token(), "comic-1".into())
        .await
        .unwrap();

    let now = OffsetDateTime::now_utc();

    let month = format!("{:04}-{:02}", now.year(), u8::from(now.month()));

    let payload = export(
        &mock,
        token(),
        "team-1".into(),
        ExportComicArchivesParams {
            months: vec![month.clone()],
        },
    )
    .await
    .unwrap();

    let stored = &mock.snapshot().comic_archives[0].archived_payload;

    assert_eq!(payload.0[&month], vec![stored.clone()]);
}

#[tokio::test]
async fn archive_rejects_non_admin_without_writing_or_deleting() {
    //
    let mock = Mock::new();

    seed_archive_scope(&mock, RoleMask::from(RoleField::TRANSLATOR));

    let archive_result =
        archive(&mock, &mock, &mock, token(), "comic-1".into()).await;

    assert_expected_variant(archive_result.unwrap_err(), ExpectedVariant::Perm);

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.comics.len(), 1);

    assert_eq!(snapshot.comic_archives.len(), 0);

    assert_eq!(snapshot.prom_records.len(), 0);
}

#[tokio::test]
async fn archive_rolls_back_when_archive_persistence_fails() {
    //
    let mock = Mock::new().with_archive_commit_failure();

    seed_archive_scope(&mock, RoleMask::from(RoleField::ADMIN));

    let archive_result =
        archive(&mock, &mock, &mock, token(), "comic-1".into()).await;

    assert!(matches!(
        archive_result,
        Err(BaseError::Unrecoverable { .. })
    ));

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.comics.len(), 1);

    assert_eq!(snapshot.chapters.len(), 1);

    assert_eq!(snapshot.assignments.len(), 1);

    assert_eq!(snapshot.assignment_invitations.len(), 1);

    assert_eq!(snapshot.pages.len(), 1);

    assert_eq!(snapshot.units.len(), 1);

    assert_eq!(snapshot.worksets[0].comic_count, 7);

    assert_eq!(snapshot.comic_archives.len(), 0);

    assert_eq!(snapshot.prom_records.len(), 0);
}
