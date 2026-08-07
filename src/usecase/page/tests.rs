// reserve_chapter_pages(reserve_chapter_pages)(positive): raw provider reserves indexed pages, chapter counters, prom checks, and PUT URLs.
// reserve_chapter_pages(process_pending)(positive): delayed completion advances raw provision after every page upload succeeds.
// reserve_chapter_pages(process_pending)(negative): delayed completion leaves raw provision pending while any upload is missing.
// reserve_chapter_pages(reserve_chapter_pages)(negative): invalid page count is rejected before state changes.
// reserve_image(reserve_image)(positive): raw provider replaces image key, resets upload state, enqueues delete and check, and returns PUT URL.
// reserve_image(reserve_image)(negative): missing page propagates an argument error.
// list_infos(list_infos)(positive): team member lists pages sorted by index with uploaded-image URLs only.
// list_infos(list_infos)(negative): non-member without assignment cannot list pages.
// get_info(get_info)(positive): team member receives one uploaded page with download URLs.
// get_info(get_info)(negative): non-member without assignment cannot read a page.
// mark_image_uploaded(mark_image_uploaded)(positive): raw provider records matching upload version without storage I/O and repeated confirmation is idempotent.
// mark_image_uploaded(mark_image_uploaded)(negative): stale upload version cannot confirm or pollute current image state.
// mark_image_uploaded(mark_image_uploaded)(negative): non-raw-provider cannot confirm upload.
// delete(delete)(positive): team admin deletes pages, enqueues image deletes, clears counters, and touches comic.
// delete(delete)(negative): non-admin delete rolls back.

use super::*;
use crate::data::instr::page::{
    ListPageInfosInstr, MarkPageImageUploadedInstr, PageImageInstr,
    ReserveChapterPagesInstr, ReservePageImageInstr,
};
use crate::value::image::{ImageExt, ImageHash};

use time::{Duration as TimeDuration, OffsetDateTime};

use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::model::read::proj::page::PageInfo;
use crate::model::read::proj::workset::WorksetInfo;
use crate::model::shared::user::UserToken;
use crate::part::prom::payload::TaskPayload;
use crate::part::prom::payload::chapter::ChapterPayload;
use crate::part::prom::payload::image::{ImagePayload, ResourceKind};
use crate::part_impl::prom::mock_impl::process_pending;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::{
    assert_expected_message, assert_expected_variant,
    assert_one_image_check_record,
};
use crate::value::chapter::{Stage, StageMask, StagePhase};
use crate::value::role::{RoleField, RoleMask};

// Reservation tests for chapter page creation and lifecycle.
mod reserve;
// Upload and deletion behavior with raw-image upload flow.
mod upload_delete;
// Validation guard tests for page operations.
mod validation;

fn token(user_id: &str) -> UserToken {
    UserToken {
        user_id: user_id.into(),
    }
}

fn workset(id: &str, team_id: &str) -> WorksetInfo {
    //
    let time = OffsetDateTime::now_utc();

    WorksetInfo {
        id: id.into(),
        team_id: team_id.into(),
        index: 0,
        name: "workset".into(),
        description: None,
        comic_count: 1,
        created_at: time,
        updated_at: time,
    }
}

fn comic(id: &str, workset_id: &str) -> ComicInfo {
    //
    let time = OffsetDateTime::now_utc();

    ComicInfo {
        id: id.into(),
        workset_id: workset_id.into(),
        index: 0,
        title: "comic".into(),
        author: "author".into(),
        description: None,
        cover_key: None,
        is_cover_uploaded: None,
        cover_version: None,
        cover_hash: None,
        cover_ext: None,
        chapter_count: 1,
        creator_id: "user-1".into(),
        workset: None,
        team: None,
        creator: None,
        last_active_at: time,
        created_at: time,
        updated_at: time,
    }
}

fn chapter(id: &str, comic_id: &str, page_count: i32) -> ChapterInfo {
    //
    let time = OffsetDateTime::now_utc();

    ChapterInfo {
        id: id.into(),
        comic_id: comic_id.into(),
        comic: None,
        is_pinned: true,
        index: 0,
        subtitle: "chapter".into(),
        page_count,
        total_unit_count: 7,
        translated_unit_count: 5,
        proofread_unit_count: 3,
        stages: StageMask::try_from(0u32).ok().unwrap(),
        creator_id: "user-1".into(),
        creator: None,
        created_at: time,
        updated_at: time,
    }
}

fn member(user_id: &str, role_mask: RoleMask) -> MemberInfo {
    MemberInfo {
        id: format!("member-{}", user_id),
        user_id: user_id.into(),
        user_nickname: user_id.into(),
        user_last_active_at: OffsetDateTime::now_utc(),
        team_id: "team-1".into(),
        user: None,
        team: None,
        roles: role_mask,
    }
}

fn assignment(
    chapter_id: &str,
    user_id: &str,
    role_mask: RoleMask,
) -> AssignmentInfo {
    //
    let time = OffsetDateTime::now_utc();

    AssignmentInfo {
        id: format!("assignment-{}-{}", chapter_id, user_id),
        chapter_id: chapter_id.into(),
        user_id: user_id.into(),
        user: None,
        chapter: None,
        roles: role_mask,
        created_at: time,
        updated_at: time,
    }
}

fn page(
    id: &str,
    index: i32,
    image_key: Option<&str>,
    image_uploaded: bool,
    image_version: u32,
) -> PageInfo {
    //
    let time = OffsetDateTime::now_utc();

    PageInfo {
        id: id.into(),
        chapter_id: "chapter-1".into(),
        index,
        image_key: image_key.map(Into::into),
        is_image_uploaded: Some(image_uploaded),
        image_version: Some(image_version),
        image_hash: Some(ImageHash::new([0u8; 32])),
        image_ext: Some(ImageExt::Png),
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        created_at: time,
        updated_at: time,
    }
}

fn seed_scope(mock: &Mock) {
    //
    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_comic(comic("comic-1", "workset-1"));

    mock.seed_chapter(chapter("chapter-1", "comic-1", 0));
}

#[tokio::test]
async fn reserve_image_replaces_key_and_enqueues_prom() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_page(page("page-1", 0, Some("old.png"), true, 1));

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::RAW_PROVIDER),
    ));

    let reserved = reserve_image(
        (&mock, &mock, &mock, &mock),
        token("user-1"),
        "page-1".into(),
        ReservePageImageInstr {
            image_hash: ImageHash::new([1u8; 32]),
            new_byte_len: 8192,
            ext: ImageExt::Jpg,
        },
    )
    .await;

    assert!(reserved.is_ok());

    let reserved = reserved.ok().unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(reserved.page_id, "page-1");

    assert_eq!(reserved.slot.as_ref().unwrap().image_version, 2);

    assert_eq!(
        reserved.slot.as_ref().unwrap().put_url,
        "https://test.local/put/page/chapter_chapter-1/page-1-2.jpg"
    );

    assert_eq!(
        snapshot.pages[0].image_key,
        Some("page/chapter_chapter-1/page-1-2.jpg".into())
    );

    assert_ne!(snapshot.pages[0].is_image_uploaded, Some(true));

    assert_eq!(snapshot.pages[0].image_version, Some(2));

    assert_eq!(snapshot.prom_records.len(), 3);

    assert!(matches!(
        snapshot.prom_records[0].payload(),
        TaskPayload::Image(ImagePayload::Delete { object_key }) if object_key == "old.png"
    ));

    assert_one_image_check_record(
        &snapshot.prom_records,
        ResourceKind::PageImage,
        "page-1",
        "page/chapter_chapter-1/page-1-2.jpg",
        2,
    );
}

#[tokio::test]
async fn reserve_image_reuses_same_uploaded_identity_without_version_bump() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_page(page("page-1", 0, Some("same.png"), true, 4));

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::RAW_PROVIDER),
    ));

    let reserved = reserve_image(
        (&mock, &mock, &mock, &mock),
        token("user-1"),
        "page-1".into(),
        ReservePageImageInstr {
            image_hash: ImageHash::new([0; 32]),
            new_byte_len: 4096,
            ext: ImageExt::Png,
        },
    )
    .await
    .unwrap();

    assert!(reserved.slot.is_none());

    assert_eq!(mock.snapshot().pages[0].image_version, Some(4));

    assert!(mock.snapshot().prom_records.is_empty());
}

#[tokio::test]
async fn reserve_image_resigns_same_pending_identity() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_page(page("page-1", 0, Some("same.png"), false, 4));

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::RAW_PROVIDER),
    ));

    let reserved = reserve_image(
        (&mock, &mock, &mock, &mock),
        token("user-1"),
        "page-1".into(),
        ReservePageImageInstr {
            image_hash: ImageHash::new([0; 32]),
            new_byte_len: 4096,
            ext: ImageExt::Png,
        },
    )
    .await
    .unwrap();

    assert_eq!(reserved.slot.as_ref().unwrap().image_version, 4);

    assert!(
        reserved
            .slot
            .as_ref()
            .unwrap()
            .put_url
            .ends_with("/same.png")
    );

    assert_eq!(mock.snapshot().pages[0].image_version, Some(4));

    assert_one_image_check_record(
        &mock.snapshot().prom_records,
        ResourceKind::PageImage,
        "page-1",
        "same.png",
        4,
    );
}

#[tokio::test]
async fn reserve_image_replaces_same_hash_with_different_extension() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_page(page("page-1", 0, Some("same.png"), true, 4));

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::RAW_PROVIDER),
    ));

    let reserved = reserve_image(
        (&mock, &mock, &mock, &mock),
        token("user-1"),
        "page-1".into(),
        ReservePageImageInstr {
            image_hash: ImageHash::new([0; 32]),
            new_byte_len: 4097,
            ext: ImageExt::Webp,
        },
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(reserved.slot.as_ref().unwrap().image_version, 5);

    assert_eq!(snapshot.pages[0].image_version, Some(5));

    assert_eq!(snapshot.pages[0].image_ext, Some(ImageExt::Webp));

    assert!(snapshot.prom_records.iter().any(|record| {
        matches!(
            record.payload(),
            TaskPayload::Image(ImagePayload::Delete { object_key })
                if object_key == "same.png"
        )
    }));

    assert_one_image_check_record(
        &snapshot.prom_records,
        ResourceKind::PageImage,
        "page-1",
        snapshot.pages[0].image_key.as_deref().unwrap(),
        5,
    );
}

#[tokio::test]
async fn reserve_image_rejects_missing_page() {
    //
    let mock = Mock::new();

    let err = reserve_image(
        (&mock, &mock, &mock, &mock),
        token("user-1"),
        "missing".into(),
        ReservePageImageInstr {
            image_hash: ImageHash::new([1u8; 32]),
            new_byte_len: 8192,
            ext: ImageExt::Jpg,
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
}

#[tokio::test]
async fn list_infos_sorts_and_resolves_uploaded_url() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_member(member("user-1", RoleMask::from(RoleField::TRANSLATOR)));

    mock.seed_page(page("page-2", 2, Some("two.png"), true, 1));

    mock.seed_page(page("page-1", 1, Some("one.png"), false, 1));

    let list = list_infos(
        (&mock, &mock),
        token("user-1"),
        ListPageInfosInstr {
            chapter_id: "chapter-1".into(),
        },
    )
    .await;

    assert!(list.is_ok());

    let list = list.ok().unwrap();

    assert_eq!(list.len(), 2);

    assert_eq!(list[0].id, "page-1");

    assert_eq!(list[0].image_url, None);

    assert_eq!(list[0].image_thumbnail_url, None);

    assert_eq!(
        list[1].image_url,
        Some("https://test.local/get/two.png".into())
    );

    assert_eq!(
        list[1].image_thumbnail_url,
        Some("https://test.local/cdn-cgi/image/width=300,fit=scale-down,quality=80,format=auto,metadata=none/two.png".into())
    );
}

#[tokio::test]
async fn list_infos_rejects_non_member_without_assignment() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    let err = list_infos(
        (&mock, &mock),
        token("user-1"),
        ListPageInfosInstr {
            chapter_id: "chapter-1".into(),
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}

#[tokio::test]
async fn get_info_resolves_uploaded_urls() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_member(member("user-1", RoleMask::from(RoleField::TRANSLATOR)));

    mock.seed_page(page("page-1", 0, Some("one.png"), true, 1));

    let found = get_info((&mock, &mock), token("user-1"), "page-1".into())
        .await
        .unwrap();

    assert_eq!(found.id, "page-1");

    assert_eq!(
        found.image_url,
        Some("https://test.local/get/one.png".into())
    );

    assert_eq!(
        found.image_thumbnail_url,
        Some("https://test.local/cdn-cgi/image/width=300,fit=scale-down,quality=80,format=auto,metadata=none/one.png".into())
    );
}

#[tokio::test]
async fn get_info_rejects_non_member_without_assignment() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_page(page("page-1", 0, Some("one.png"), true, 1));

    let err = get_info((&mock, &mock), token("user-1"), "page-1".into())
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}

#[tokio::test]
async fn mark_image_uploaded_marks_once_and_idempotent() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_page(page("page-1", 0, Some("one.png"), false, 2));

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::RAW_PROVIDER),
    ));

    let first = mark_image_uploaded(
        (&mock, &mock, &mock),
        token("user-1"),
        "page-1".into(),
        MarkPageImageUploadedInstr { image_version: 2 },
    )
    .await;

    assert!(first.is_ok());

    let second = mark_image_uploaded(
        (&mock, &mock, &mock),
        token("user-1"),
        "page-1".into(),
        MarkPageImageUploadedInstr { image_version: 2 },
    )
    .await;

    assert!(second.is_ok());

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.pages[0].is_image_uploaded, Some(true));
}
