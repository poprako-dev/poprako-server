use super::super::mark_image_uploaded;

use poprako_obj_dept::key::ObjKey;
use poprako_obj_dept::model::meta::ObjMeta;
use time::OffsetDateTime;

use crate::data::instr::page_source::MarkPageSourceImageUploadedInstr;
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::page_source::PageSourceInfo;
use crate::model::shared::user::UserToken;
use crate::part_impl::repo::mock_impl::{Mock, MockContext, MockObjRecord};
use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;
use crate::value::role::{RoleField, RoleMask};

fn seed_scope(mock: &Mock) {
    super::seed_page_source_scope(mock, 1);

    let time = OffsetDateTime::now_utc();

    mock.seed_page(PageSourceInfo {
        id: "page_source--00000000001".into(),
        chapter_id: "chapter--00000000001".into(),
        index: 0,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        created_at: time,
        updated_at: time,
    });

    mock.seed_assignment(AssignmentInfo {
        id: "assignment--00000000001".into(),
        chapter_id: "chapter--00000000001".into(),
        user_id: "user--00000000001".into(),
        user: None,
        chapter: None,
        roles: RoleMask::from(RoleField::RAW_PROVIDER),
        created_at: time,
        updated_at: time,
    });

    let key = ObjKey {
        id: "page_source--00000000001".into(),
        ver: 3,
        image:
            "page_source/chapter--00000000001/page_source--00000000001/3.png"
                .into(),
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
        .entry("page_source_image")
        .or_default()
        .insert(
            "page_source--00000000001".into(),
            MockObjRecord {
                version: 3,
                meta: Some(meta),
            },
        );
}

async fn mark(mock: &Mock, version: u32) -> crate::result::BaseRest<()> {
    mark_image_uploaded::<_, MockContext, _, _>(
        (mock, mock, mock),
        UserToken {
            user_id: "user--00000000001".into(),
        },
        "page_source--00000000001".into(),
        MarkPageSourceImageUploadedInstr { image_ver: version },
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
        mock.snapshot()
            .objs
            .get("page_source_image")
            .unwrap()
            .get("page_source--00000000001")
            .unwrap()
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

    let error = mark(&mock, 2).await.err().unwrap();

    assert_expected_variant(&error, ExpectedVariant::Args);

    assert!(
        !mock
            .snapshot()
            .objs
            .get("page_source_image")
            .unwrap()
            .get("page_source--00000000001")
            .unwrap()
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

    assert_expected_variant(&stale_error, ExpectedVariant::Args);
    assert!(
        !mock
            .snapshot()
            .objs
            .get("page_source_image")
            .unwrap()
            .get("page_source--00000000001")
            .unwrap()
            .meta
            .as_ref()
            .unwrap()
            .is_avail
    );

    mark(&mock, 3).await.unwrap();

    assert!(
        mock.snapshot()
            .objs
            .get("page_source_image")
            .unwrap()
            .get("page_source--00000000001")
            .unwrap()
            .meta
            .as_ref()
            .unwrap()
            .is_avail
    );
}

#[tokio::test]
async fn translator_cannot_mark_page_source_image_uploaded() {
    let mock = Mock::new();

    seed_scope(&mock);

    mock.state
        .lock()
        .unwrap()
        .assignments
        .get_mut(0)
        .unwrap()
        .roles = RoleMask::from(RoleField::TRANSLATOR);

    let error = mark(&mock, 3).await.err().unwrap();

    assert_expected_variant(&error, ExpectedVariant::Perm);
    assert!(
        !mock
            .snapshot()
            .objs
            .get("page_source_image")
            .unwrap()
            .get("page_source--00000000001")
            .unwrap()
            .meta
            .as_ref()
            .unwrap()
            .is_avail
    );
}

#[tokio::test]
async fn tombstoned_chapter_cannot_mark_page_source_image_uploaded() {
    let mock = Mock::new();

    seed_scope(&mock);

    mock.state
        .lock()
        .unwrap()
        .deleted_chapter_ids
        .insert("chapter--00000000001".into());

    let error = mark(&mock, 3).await.err().unwrap();

    assert_expected_variant(&error, ExpectedVariant::Args);

    assert!(
        !mock
            .snapshot()
            .objs
            .get("page_source_image")
            .unwrap()
            .get("page_source--00000000001")
            .unwrap()
            .meta
            .as_ref()
            .unwrap()
            .is_avail
    );
}

// mark_image_uploaded(positive): reviewers share the source upload capability.
#[tokio::test]
async fn reviewer_can_confirm_source_images() {
    let mock = Mock::new();

    seed_scope(&mock);

    mock.state
        .lock()
        .unwrap()
        .assignments
        .first_mut()
        .unwrap()
        .roles = RoleMask::from(RoleField::REVIEWER);

    mark(&mock, 3).await.unwrap();

    assert!(
        mock.snapshot()
            .objs
            .get("page_source_image")
            .unwrap()
            .get("page_source--00000000001")
            .unwrap()
            .meta
            .as_ref()
            .unwrap()
            .is_avail
    );
}

// mark_image_uploaded(negative): an activity update failure rolls back the object confirmation.
#[tokio::test]
async fn failed_activity_update_rolls_back_confirmation() {
    let mock = Mock::new();

    seed_scope(&mock);

    mock.state.lock().unwrap().comics.clear();

    assert!(mark(&mock, 3).await.is_err());

    assert!(
        !mock
            .snapshot()
            .objs
            .get("page_source_image")
            .unwrap()
            .get("page_source--00000000001")
            .unwrap()
            .meta
            .as_ref()
            .unwrap()
            .is_avail
    );
}
