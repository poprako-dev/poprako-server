use super::super::view::page_info_views;
use super::*;

use time::OffsetDateTime;

use poprako_obj_dept::key::ObjKey;
use poprako_obj_dept::model::meta::ObjMeta;

use crate::data::instr::page::ListPageInfosInstr;
use crate::model::read::proj::page::PageInfo;
use crate::part_impl::repo::mock_impl::{Mock, MockObjRecord};
use crate::result::{BaseError, ExpectedVariant};
use crate::test_util::assert_expected_variant;
use crate::usecase::page::list::{get_info, list_infos};
use crate::value::image::{ImageExt, ImageHash};
use crate::value::page::MAX_CHAPTER_PAGE_COUNT;
use crate::value::role::RoleField;

fn page(id: &str, index: usize) -> PageInfo {
    let time = OffsetDateTime::now_utc();

    PageInfo {
        id: id.into(),
        chapter_id: "chapter-1".into(),
        index,
        total_unit_count: 3,
        translated_unit_count: 2,
        proofread_unit_count: 1,
        created_at: time,
        updated_at: time,
    }
}

fn page_image(
    id: &str,
    version: u32,
    uploaded: bool,
    hash: u8,
    ext: &str,
) -> ObjMeta {
    ObjMeta {
        key: ObjKey {
            id: id.into(),
            ver: version,
            image: format!("page/chapter_test/{}-{}.{}", id, version, ext),
        },
        is_avail: uploaded,
        hash: vec![hash; 32],
        ext: ext.into(),
    }
}

#[tokio::test]
async fn page_views_keep_each_image_url_with_its_metadata_snapshot() {
    let mock = Mock::new();

    let uploaded_meta = page_image("page-uploaded", 7, true, 7, "png");

    let pending_meta = page_image("page-pending", 4, false, 4, "jpg");

    let mut state = mock.state.lock().unwrap();

    let page_images = state.objs.entry("page_image").or_default();

    page_images.insert(
        "page-uploaded".into(),
        MockObjRecord {
            version: uploaded_meta.key.ver,
            meta: Some(uploaded_meta),
        },
    );

    page_images.insert(
        "page-pending".into(),
        MockObjRecord {
            version: pending_meta.key.ver,
            meta: Some(pending_meta),
        },
    );

    drop(state);

    let page_models = vec![
        page("page-uploaded", 0),
        page("page-pending", 1),
        page("page-missing", 2),
    ];

    let page_views = page_info_views(&mock, page_models).await.unwrap();

    assert_eq!(page_views.len(), 3);

    let uploaded_view = &page_views[0];

    assert_eq!(
        uploaded_view.image_url.as_deref(),
        Some("https://obj.test/page/chapter_test/page-uploaded-7.png")
    );

    assert_eq!(
        uploaded_view.image_optimized_url.as_deref(),
        Some(
            "https://obj.test/optimized/page/chapter_test/page-uploaded-7.png"
        )
    );

    assert_eq!(
        uploaded_view.image_thumbnail_url.as_deref(),
        Some(
            "https://obj.test/thumbnail/page/chapter_test/page-uploaded-7.png"
        )
    );

    assert_eq!(uploaded_view.image_hash, Some(ImageHash::new([7; 32])));

    assert_eq!(uploaded_view.ext, Some(ImageExt::Png));

    let pending_view = &page_views[1];

    assert_eq!(pending_view.image_url, None);

    assert_eq!(pending_view.image_optimized_url, None);

    assert_eq!(pending_view.image_thumbnail_url, None);

    assert_eq!(pending_view.image_hash, Some(ImageHash::new([4; 32])));

    assert_eq!(pending_view.ext, Some(ImageExt::Jpg));

    let missing_view = &page_views[2];

    assert_eq!(missing_view.image_url, None);

    assert_eq!(missing_view.image_optimized_url, None);

    assert_eq!(missing_view.image_thumbnail_url, None);

    assert_eq!(missing_view.image_hash, None);

    assert_eq!(missing_view.ext, None);
}

#[tokio::test]
async fn list_infos_sorts_pages_and_resolves_only_available_image_urls() {
    let mock = Mock::new();

    seed_page_scope(&mock, 2);

    mock.seed_member(page_member(
        "user-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));
    mock.seed_page(page("page-2", 2));
    mock.seed_page(page("page-1", 1));

    seed_page_obj(&mock, "page-2", 2, true, 2, ImageExt::Png);
    seed_page_obj(&mock, "page-1", 1, false, 1, ImageExt::Jpg);

    let pages = list_infos(
        (&mock, &mock),
        page_token("user-1"),
        ListPageInfosInstr {
            chapter_id: "chapter-1".into(),
        },
    )
    .await
    .unwrap();

    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0].id, "page-1");
    assert_eq!(pages[0].image_url, None);
    assert_eq!(pages[0].image_optimized_url, None);
    assert_eq!(pages[0].image_thumbnail_url, None);
    assert_eq!(pages[1].id, "page-2");
    assert_eq!(
        pages[1].image_url.as_deref(),
        Some("https://obj.test/page/chapter_chapter-1/page-2-2.png")
    );
    assert_eq!(
        pages[1].image_optimized_url.as_deref(),
        Some("https://obj.test/optimized/page/chapter_chapter-1/page-2-2.png")
    );
    assert_eq!(
        pages[1].image_thumbnail_url.as_deref(),
        Some("https://obj.test/thumbnail/page/chapter_chapter-1/page-2-2.png")
    );
}

#[tokio::test]
async fn list_infos_returns_complete_manifest_at_business_maximum() {
    let mock = Mock::new();

    seed_page_scope(&mock, MAX_CHAPTER_PAGE_COUNT);

    mock.seed_member(page_member(
        "user-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    for index in (0..MAX_CHAPTER_PAGE_COUNT).rev() {
        mock.seed_page(page(&format!("page-{index:03}"), index));
    }

    let pages = list_infos(
        (&mock, &mock),
        page_token("user-1"),
        ListPageInfosInstr {
            chapter_id: "chapter-1".into(),
        },
    )
    .await
    .unwrap();

    assert_eq!(pages.len(), MAX_CHAPTER_PAGE_COUNT);

    assert_eq!(pages.first().unwrap().id, "page-000");

    assert_eq!(pages.last().unwrap().id, "page-199");
}

#[tokio::test]
async fn list_infos_rejects_non_member_without_assignment() {
    let mock = Mock::new();

    seed_page_scope(&mock, 0);

    let error = list_infos(
        (&mock, &mock),
        page_token("user-1"),
        ListPageInfosInstr {
            chapter_id: "chapter-1".into(),
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(error, ExpectedVariant::Perm);
}

#[tokio::test]
async fn list_infos_rejects_persisted_page_count_above_business_maximum() {
    let mock = Mock::new();

    seed_page_scope(&mock, MAX_CHAPTER_PAGE_COUNT + 1);

    mock.seed_member(page_member(
        "user-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    for index in 0..=MAX_CHAPTER_PAGE_COUNT {
        mock.seed_page(page(&format!("page-{index:03}"), index));
    }

    let error = list_infos(
        (&mock, &mock),
        page_token("user-1"),
        ListPageInfosInstr {
            chapter_id: "chapter-1".into(),
        },
    )
    .await
    .unwrap_err();

    assert!(matches!(error, BaseError::Unrecoverable { .. }));
}

#[tokio::test]
async fn get_info_resolves_available_image_metadata_and_urls() {
    let mock = Mock::new();

    seed_page_scope(&mock, 1);

    mock.seed_member(page_member(
        "user-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));
    mock.seed_page(page("page-1", 0));

    seed_page_obj(&mock, "page-1", 7, true, 7, ImageExt::Png);

    let found = get_info((&mock, &mock), page_token("user-1"), "page-1".into())
        .await
        .unwrap();

    assert_eq!(found.id, "page-1");
    assert_eq!(found.image_hash, Some(ImageHash::new([7; 32])));
    assert_eq!(found.ext, Some(ImageExt::Png));
    assert_eq!(
        found.image_url.as_deref(),
        Some("https://obj.test/page/chapter_chapter-1/page-1-7.png")
    );
    assert_eq!(
        found.image_optimized_url.as_deref(),
        Some("https://obj.test/optimized/page/chapter_chapter-1/page-1-7.png")
    );
    assert_eq!(
        found.image_thumbnail_url.as_deref(),
        Some("https://obj.test/thumbnail/page/chapter_chapter-1/page-1-7.png")
    );
}

#[tokio::test]
async fn get_info_rejects_non_member_without_assignment() {
    let mock = Mock::new();

    seed_page_scope(&mock, 1);

    mock.seed_page(page("page-1", 0));

    let error = get_info((&mock, &mock), page_token("user-1"), "page-1".into())
        .await
        .err()
        .unwrap();

    assert_expected_variant(error, ExpectedVariant::Perm);
}
