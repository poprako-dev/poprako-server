use super::super::mark_image_uploaded;

use poprako_obj_dept::key::ObjKey;
use poprako_obj_dept::model::meta::ObjMeta;
use time::OffsetDateTime;

use crate::data::instr::page::MarkPageImageUploadedInstr;
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::page::PageInfo;
use crate::model::shared::user::UserToken;
use crate::part_impl::repo::mock_impl::{Mock, MockContext, MockObjRecord};
use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;
use crate::value::role::{RoleField, RoleMask};

fn seed_scope(mock: &Mock) {
    let time = OffsetDateTime::now_utc();

    mock.seed_page(PageInfo {
        id: "page-1".into(),
        chapter_id: "chapter-1".into(),
        index: 0,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        created_at: time,
        updated_at: time,
    });

    mock.seed_assignment(AssignmentInfo {
        id: "assignment-1".into(),
        chapter_id: "chapter-1".into(),
        user_id: "user-1".into(),
        user: None,
        chapter: None,
        roles: RoleMask::from(RoleField::RAW_PROVIDER),
        created_at: time,
        updated_at: time,
    });

    let key = ObjKey {
        id: "page-1".into(),
        ver: 3,
        image: "page/chapter_chapter-1/page-1-3.png".into(),
    };

    let meta = ObjMeta {
        key,
        is_avail: false,
        hash: vec![0; 32],
        ext: "png".into(),
    };

    mock.state
        .lock()
        .unwrap()
        .objs
        .entry("page_image")
        .or_default()
        .insert(
            "page-1".into(),
            MockObjRecord {
                version: 3,
                meta: Some(meta),
            },
        );
}

async fn mark(mock: &Mock, version: u32) -> crate::result::BaseRest<()> {
    mark_image_uploaded::<MockContext, _, _>(
        (mock, mock),
        UserToken {
            user_id: "user-1".into(),
        },
        "page-1".into(),
        MarkPageImageUploadedInstr { image_ver: version },
    )
    .await
}

#[tokio::test]
async fn current_generation_is_marked_idempotently() {
    let mock = Mock::new();

    seed_scope(&mock);

    mark(&mock, 3).await.unwrap();

    mark(&mock, 3).await.unwrap();

    assert!(
        mock.snapshot().objs["page_image"]["page-1"]
            .meta
            .as_ref()
            .unwrap()
            .is_avail
    );
}

#[tokio::test]
async fn stale_generation_does_not_mark_current() {
    let mock = Mock::new();

    seed_scope(&mock);

    let err = mark(&mock, 2).await.err().unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);

    assert!(
        !mock.snapshot().objs["page_image"]["page-1"]
            .meta
            .as_ref()
            .unwrap()
            .is_avail
    );
}

#[tokio::test]
async fn stale_replay_is_rejected_before_current_generation_is_accepted() {
    let mock = Mock::new();

    seed_scope(&mock);

    let stale_error = mark(&mock, 2).await.err().unwrap();

    assert_expected_variant(stale_error, ExpectedVariant::Args);
    assert!(
        !mock.snapshot().objs["page_image"]["page-1"]
            .meta
            .as_ref()
            .unwrap()
            .is_avail
    );

    mark(&mock, 3).await.unwrap();

    assert!(
        mock.snapshot().objs["page_image"]["page-1"]
            .meta
            .as_ref()
            .unwrap()
            .is_avail
    );
}

#[tokio::test]
async fn non_raw_provider_cannot_mark_page_image_uploaded() {
    let mock = Mock::new();

    seed_scope(&mock);

    mock.state.lock().unwrap().assignments[0].roles =
        RoleMask::from(RoleField::REVIEWER);

    let error = mark(&mock, 3).await.err().unwrap();

    assert_expected_variant(error, ExpectedVariant::Perm);
    assert!(
        !mock.snapshot().objs["page_image"]["page-1"]
            .meta
            .as_ref()
            .unwrap()
            .is_avail
    );
}

#[tokio::test]
async fn tombstoned_chapter_cannot_mark_page_image_uploaded() {
    let mock = Mock::new();

    seed_scope(&mock);

    mock.state
        .lock()
        .unwrap()
        .deleted_chapter_ids
        .insert("chapter-1".into());

    let error = mark(&mock, 3).await.err().unwrap();

    assert_expected_variant(error, ExpectedVariant::Args);

    assert!(
        !mock.snapshot().objs["page_image"]["page-1"]
            .meta
            .as_ref()
            .unwrap()
            .is_avail
    );
}
