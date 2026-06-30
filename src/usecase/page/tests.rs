// reserve_chapter_pages(reserve_chapter_pages)(positive): raw provider reserves indexed pages, chapter counters, prom checks, and PUT URLs.
// reserve_chapter_pages(reserve_chapter_pages)(negative): invalid page count is rejected before state changes.
// reserve_image(reserve_image)(positive): raw provider replaces image key, resets upload state, enqueues delete and check, and returns PUT URL.
// reserve_image(reserve_image)(negative): missing page propagates an argument error.
// list_infos(list_infos)(positive): team member lists pages sorted by index with uploaded-image URLs only.
// list_infos(list_infos)(negative): non-member without assignment cannot list pages.
// mark_image_uploaded(mark_image_uploaded)(positive): raw provider confirms matching upload version and repeated confirmation is idempotent.
// mark_image_uploaded(mark_image_uploaded)(negative): stale upload version cannot confirm or pollute current image state.
// mark_image_uploaded(mark_image_uploaded)(negative): non-raw-provider cannot confirm upload.
// delete(delete)(positive): team admin deletes pages, enqueues image deletes, clears counters, and touches comic.
// delete(delete)(negative): non-admin delete rolls back.

use super::*;

use time::OffsetDateTime;

use crate::model::assignment::AssignmentInfo;
use crate::model::chapter::ChapterInfo;
use crate::model::comic::ComicInfo;
use crate::model::member::MemberInfo;
use crate::model::page::PageInfo;
use crate::model::role::{RoleField, RoleMask};
use crate::model::workset::WorksetInfo;
use crate::part::prom::Payload;
use crate::part::prom::intention::{ImageIntention, ImageKind};
use crate::part_impl::repo_mock::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::{
    assert_expected_message, assert_expected_variant, assert_one_image_check_record,
};
use crate::value::chapter::WorkflowStageMask;

fn token(user_id: &str) -> UserToken {
    UserToken {
        user_id: user_id.into(),
    }
}

fn workset(id: &str, team_id: &str) -> WorksetInfo {
    let time = OffsetDateTime::now_utc();

    WorksetInfo {
        id: id.into(),
        team_id: team_id.into(),
        index: 0,
        name: "workset".into(),
        description: None,
        comic_count: 1,
        comic_next_index: 1,
        created_at: time,
        updated_at: time,
    }
}

fn comic(id: &str, workset_id: &str) -> ComicInfo {
    let time = OffsetDateTime::now_utc();

    ComicInfo {
        id: id.into(),
        workset_id: workset_id.into(),
        index: 0,
        title: "comic".into(),
        author: "author".into(),
        description: None,
        is_completed: false,
        cover_key: None,
        cover_uploaded: false,
        cover_version: 0,
        chapter_count: 1,
        chapter_next_index: 1,
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
    let time = OffsetDateTime::now_utc();

    ChapterInfo {
        id: id.into(),
        comic_id: comic_id.into(),
        is_pinned: true,
        index: 0,
        subtitle: "chapter".into(),
        page_count,
        total_unit_count: 7,
        translated_unit_count: 5,
        proofread_unit_count: 3,
        stages: WorkflowStageMask::try_from(0u32).ok().unwrap(),
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
        team_id: "team-1".into(),
        user: None,
        team: None,
        roles: role_mask,
    }
}

fn assignment(chapter_id: &str, user_id: &str, role_mask: RoleMask) -> AssignmentInfo {
    let time = OffsetDateTime::now_utc();

    AssignmentInfo {
        id: format!("assignment-{}-{}", chapter_id, user_id),
        chapter_id: chapter_id.into(),
        user_id: user_id.into(),
        user: None,
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
    image_version: i64,
) -> PageInfo {
    let time = OffsetDateTime::now_utc();

    PageInfo {
        id: id.into(),
        chapter_id: "chapter-1".into(),
        index,
        image_key: image_key.map(Into::into),
        image_uploaded,
        image_version,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        created_at: time,
        updated_at: time,
    }
}

fn seed_scope(mock: &Mock) {
    mock.seed_workset(workset("workset-1", "team-1"));
    mock.seed_comic(comic("comic-1", "workset-1"));
    mock.seed_chapter(chapter("chapter-1", "comic-1", 0));
}

#[tokio::test]
async fn reserve_chapter_pages_creates_pages_and_urls() {
    let mock = Mock::new();
    seed_scope(&mock);
    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::RAW_PROVIDER),
    ));

    let reserved = reserve_chapter_pages(
        &mock,
        &mock,
        &mock,
        &mock,
        token("user-1"),
        ReserveChapterPagesData {
            chapter_id: "chapter-1".into(),
            page_count: 2,
            file_ext: "png".into(),
        },
    )
    .await;
    assert!(reserved.is_ok());
    let reserved = reserved.ok().unwrap();
    let snapshot = mock.snapshot();

    assert_eq!(reserved.creations.len(), 2);
    assert_eq!(snapshot.pages.len(), 2);
    assert_eq!(snapshot.pages[0].index, 0);
    assert_eq!(snapshot.pages[1].index, 1);
    assert_eq!(snapshot.pages[0].image_version, 1);
    assert_eq!(snapshot.pages[1].image_version, 1);
    assert_eq!(reserved.creations[0].image_version, 1);
    assert_eq!(reserved.creations[1].image_version, 1);
    assert_eq!(snapshot.chapters[0].page_count, 2);
    assert_eq!(snapshot.prom_records.len(), 2);
    assert!(
        reserved.creations[0]
            .put_url
            .contains("https://test.local/put/chapter_chapter-1/")
    );
    for creation in &reserved.creations {
        let page_info = snapshot
            .pages
            .iter()
            .find(|page_info| page_info.id == creation.page_id)
            .unwrap();
        let object_key = page_info.image_key.as_deref().unwrap();

        assert!(object_key.ends_with("-1.png"));
        assert_one_image_check_record(
            &snapshot.prom_records,
            ImageKind::PageImage,
            &creation.page_id,
            object_key,
            creation.image_version,
        );
    }
}

#[tokio::test]
async fn reserve_chapter_pages_rejects_invalid_count() {
    let mock = Mock::new();
    seed_scope(&mock);

    let err = reserve_chapter_pages(
        &mock,
        &mock,
        &mock,
        &mock,
        token("user-1"),
        ReserveChapterPagesData {
            chapter_id: "chapter-1".into(),
            page_count: 0,
            file_ext: "png".into(),
        },
    )
    .await
    .err()
    .unwrap();
    let snapshot = mock.snapshot();

    assert_expected_variant(err, ExpectedVariant::ArgsInvalid);
    assert!(snapshot.pages.is_empty());
    assert!(snapshot.prom_records.is_empty());
}

#[tokio::test]
async fn reserve_image_replaces_key_and_enqueues_prom() {
    let mock = Mock::new();
    seed_scope(&mock);
    mock.seed_page(page("page-1", 0, Some("old.png"), true, 1));
    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::RAW_PROVIDER),
    ));

    let reserved = reserve_image(
        &mock,
        &mock,
        &mock,
        &mock,
        token("user-1"),
        "page-1".into(),
        ReservePageImageData {
            file_ext: "jpg".into(),
        },
    )
    .await;
    assert!(reserved.is_ok());
    let reserved = reserved.ok().unwrap();
    let snapshot = mock.snapshot();

    assert_eq!(reserved.page_id, "page-1");
    assert_eq!(reserved.image_version, 2);
    assert_eq!(
        reserved.put_url,
        "https://test.local/put/chapter_chapter-1/page_page-1-2.jpg"
    );
    assert_eq!(
        snapshot.pages[0].image_key,
        Some("chapter_chapter-1/page_page-1-2.jpg".into())
    );
    assert!(!snapshot.pages[0].image_uploaded);
    assert_eq!(snapshot.pages[0].image_version, 2);
    assert_eq!(snapshot.prom_records.len(), 2);
    assert!(matches!(
        &snapshot.prom_records[0].payload,
        Payload::Image(ImageIntention::Delete { object_key }) if object_key == "old.png"
    ));
    assert_one_image_check_record(
        &snapshot.prom_records,
        ImageKind::PageImage,
        "page-1",
        "chapter_chapter-1/page_page-1-2.jpg",
        2,
    );
}

#[tokio::test]
async fn reserve_image_rejects_missing_page() {
    let mock = Mock::new();

    let err = reserve_image(
        &mock,
        &mock,
        &mock,
        &mock,
        token("user-1"),
        "missing".into(),
        ReservePageImageData {
            file_ext: "jpg".into(),
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::ArgsInvalid);
}

#[tokio::test]
async fn list_infos_sorts_and_resolves_uploaded_url() {
    let mock = Mock::new();
    seed_scope(&mock);
    mock.seed_member(member("user-1", RoleMask::from(RoleField::TRANSLATOR)));
    mock.seed_page(page("page-2", 2, Some("two.png"), true, 1));
    mock.seed_page(page("page-1", 1, Some("one.png"), false, 1));

    let list = list_infos(
        &mock,
        &mock,
        token("user-1"),
        ListPageInfosData {
            chapter_id: "chapter-1".into(),
            offset: 0,
            limit: 10,
        },
    )
    .await;
    assert!(list.is_ok());
    let list = list.ok().unwrap();

    assert_eq!(list.len(), 2);
    assert_eq!(list[0].id, "page-1");
    assert_eq!(list[0].image_url, None);
    assert_eq!(
        list[1].image_url,
        Some("https://test.local/get/two.png".into())
    );
}

#[tokio::test]
async fn list_infos_rejects_non_member_without_assignment() {
    let mock = Mock::new();
    seed_scope(&mock);

    let err = list_infos(
        &mock,
        &mock,
        token("user-1"),
        ListPageInfosData {
            chapter_id: "chapter-1".into(),
            offset: 0,
            limit: 10,
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::PermDeny);
}

#[tokio::test]
async fn mark_image_uploaded_marks_once_and_idempotent() {
    let mock = Mock::new();
    seed_scope(&mock);
    mock.seed_page(page("page-1", 0, Some("one.png"), false, 2));
    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::RAW_PROVIDER),
    ));

    let first = mark_image_uploaded(
        &mock,
        &mock,
        token("user-1"),
        "page-1".into(),
        MarkPageImageUploadedData { image_version: 2 },
    )
    .await;
    assert!(first.is_ok());
    let second = mark_image_uploaded(
        &mock,
        &mock,
        token("user-1"),
        "page-1".into(),
        MarkPageImageUploadedData { image_version: 2 },
    )
    .await;
    assert!(second.is_ok());
    let snapshot = mock.snapshot();

    assert!(snapshot.pages[0].image_uploaded);
}

#[tokio::test]
async fn mark_image_uploaded_rejects_stale_replay_then_accepts_current_version() {
    let mock = Mock::new();
    seed_scope(&mock);
    mock.seed_page(page(
        "page-1",
        0,
        Some("chapter_chapter-1/page_page-1-1.png"),
        true,
        1,
    ));
    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::RAW_PROVIDER),
    ));

    let reserved = reserve_image(
        &mock,
        &mock,
        &mock,
        &mock,
        token("user-1"),
        "page-1".into(),
        ReservePageImageData {
            file_ext: "png".into(),
        },
    )
    .await
    .ok()
    .unwrap();
    assert_eq!(reserved.image_version, 2);

    let err = mark_image_uploaded(
        &mock,
        &mock,
        token("user-1"),
        "page-1".into(),
        MarkPageImageUploadedData { image_version: 1 },
    )
    .await
    .err()
    .unwrap();
    let snapshot = mock.snapshot();

    assert_expected_message(
        err,
        ExpectedVariant::ArgsInvalid,
        "error-stale-page-image-upload",
    );
    assert!(!snapshot.pages[0].image_uploaded);
    assert_eq!(snapshot.pages[0].image_version, 2);

    mark_image_uploaded(
        &mock,
        &mock,
        token("user-1"),
        "page-1".into(),
        MarkPageImageUploadedData { image_version: 2 },
    )
    .await
    .unwrap();
    assert!(mock.snapshot().pages[0].image_uploaded);
}

#[tokio::test]
async fn mark_image_uploaded_rejects_non_raw_provider() {
    let mock = Mock::new();
    seed_scope(&mock);
    mock.seed_page(page("page-1", 0, Some("one.png"), false, 1));
    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::REVIEWER),
    ));

    let err = mark_image_uploaded(
        &mock,
        &mock,
        token("user-1"),
        "page-1".into(),
        MarkPageImageUploadedData { image_version: 1 },
    )
    .await
    .err()
    .unwrap();
    let snapshot = mock.snapshot();

    assert_expected_variant(err, ExpectedVariant::PermDeny);
    assert!(!snapshot.pages[0].image_uploaded);
}

#[tokio::test]
async fn delete_by_chapter_deletes_pages_and_clears_counters() {
    let mock = Mock::new();
    seed_scope(&mock);
    mock.seed_member(member("user-1", RoleMask::from(RoleField::ADMIN)));
    mock.seed_page(page("page-1", 0, Some("one.png"), true, 1));
    mock.seed_page(page("page-2", 1, None, false, 0));

    let deleted = delete(&mock, &mock, &mock, token("user-1"), "chapter-1".into()).await;
    assert!(deleted.is_ok());
    let snapshot = mock.snapshot();

    assert!(snapshot.pages.is_empty());
    assert_eq!(snapshot.chapters[0].page_count, 0);
    assert_eq!(snapshot.chapters[0].total_unit_count, 0);
    assert_eq!(snapshot.prom_records.len(), 1);
    assert!(matches!(
        &snapshot.prom_records[0].payload,
        Payload::Image(ImageIntention::Delete { object_key }) if object_key == "one.png"
    ));
}

#[tokio::test]
async fn delete_by_chapter_rejects_non_admin_and_rolls_back() {
    let mock = Mock::new();
    seed_scope(&mock);
    mock.seed_member(member("user-1", RoleMask::from(RoleField::TRANSLATOR)));
    mock.seed_page(page("page-1", 0, Some("one.png"), true, 1));

    let err = delete(&mock, &mock, &mock, token("user-1"), "chapter-1".into())
        .await
        .err()
        .unwrap();
    let snapshot = mock.snapshot();

    assert_expected_variant(err, ExpectedVariant::PermDeny);
    assert_eq!(snapshot.pages.len(), 1);
    assert_eq!(snapshot.chapters[0].page_count, 0);
    assert!(snapshot.prom_records.is_empty());
}
