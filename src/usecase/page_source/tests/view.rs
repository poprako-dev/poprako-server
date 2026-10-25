use super::super::view::page_source_info_views;
use super::*;

use time::OffsetDateTime;

use poprako_obj_dept::key::ObjKey;
use poprako_obj_dept::model::meta::ObjMeta;

use crate::data::instr::page_source::ListPageSourceInfosInstr;
use crate::model::read::proj::page_source::PageSourceInfo;
use crate::part_impl::repo::mock_impl::{Mock, MockObjRecord};
use crate::result::{BaseError, ExpectedVariant};
use crate::test_util::assert_expected_variant;
use crate::usecase::page_source::list::{get_info, list_infos};
use crate::value::image::{ImageExt, ImageHash};
use crate::value::page_source::MAX_CHAPTER_PAGE_SOURCE_COUNT;
use crate::value::role::RoleField;

fn page(id: &str, index: usize) -> PageSourceInfo {
    let time = OffsetDateTime::now_utc();

    PageSourceInfo {
        id: id.into(),
        chapter_id: "chapter--00000000001".into(),
        index,
        total_unit_count: 3,
        translated_unit_count: 2,
        proofread_unit_count: 1,
        created_at: time,
        updated_at: time,
    }
}

fn page_source_image(
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
            image: [
                "page_source/chapter--00000000001/",
                id,
                "/",
                &version.to_string(),
                ".",
                ext,
            ]
            .concat(),
        },
        is_avail: uploaded,
        hash: vec![hash; 32],
        ext: ext.into(),
    }
}

#[tokio::test]
async fn page_source_views_keep_each_image_url_with_its_metadata_snapshot() {
    let mock = Mock::new();

    let uploaded_meta =
        page_source_image("page_source--04707503927", 7, true, 7, "png");

    let pending_meta =
        page_source_image("page_source--47740658756", 4, false, 4, "jpg");

    {
        let mut state = mock.state.lock().unwrap();

        let page_source_images =
            state.objs.entry("page_source_image").or_default();

        page_source_images.insert(
            "page_source--04707503927".into(),
            MockObjRecord {
                version: uploaded_meta.key.ver,
                meta: Some(uploaded_meta),
            },
        );

        page_source_images.insert(
            "page_source--47740658756".into(),
            MockObjRecord {
                version: pending_meta.key.ver,
                meta: Some(pending_meta),
            },
        );

        drop(state);
    }

    let page_source_models = vec![
        page("page_source--04707503927", 0),
        page("page_source--47740658756", 1),
        page("page_source--66314807403", 2),
    ];

    let page_source_views =
        page_source_info_views((&mock, &mock), page_source_models)
            .await
            .unwrap();

    assert_eq!(page_source_views.len(), 3);

    let uploaded_view = page_source_views.first().unwrap();

    assert_eq!(
        uploaded_view.image_url.as_deref(),
        Some(
            "https://obj.test/page_source/chapter--00000000001/\
                page_source--04707503927/7.png"
        )
    );

    assert_eq!(
        uploaded_view.image_optimized_url.as_deref(),
        Some(
            "https://obj.test/optimized/page_source/\
                chapter--00000000001/page_source--04707503927/7.png"
        )
    );

    assert_eq!(
        uploaded_view.image_thumbnail_url.as_deref(),
        Some(
            "https://obj.test/thumbnail/page_source/\
                chapter--00000000001/page_source--04707503927/7.png"
        )
    );

    assert_eq!(uploaded_view.image_hash, Some(ImageHash::new([7; 32])));

    assert_eq!(uploaded_view.ext, Some(ImageExt::Png));

    let pending_view = page_source_views.get(1).unwrap();

    assert_eq!(pending_view.image_url, None);

    assert_eq!(pending_view.image_optimized_url, None);

    assert_eq!(pending_view.image_thumbnail_url, None);

    assert_eq!(pending_view.image_hash, Some(ImageHash::new([4; 32])));

    assert_eq!(pending_view.ext, Some(ImageExt::Jpg));

    let missing_view = page_source_views.get(2).unwrap();

    assert_eq!(missing_view.image_url, None);

    assert_eq!(missing_view.image_optimized_url, None);

    assert_eq!(missing_view.image_thumbnail_url, None);

    assert_eq!(missing_view.image_hash, None);

    assert_eq!(missing_view.ext, None);
}

#[tokio::test]
async fn list_infos_sorts_page_sources_and_resolves_only_available_image_urls()
{
    let mock = Mock::new();

    seed_page_source_scope(&mock, 2);

    mock.seed_member(page_source_member(
        "user--00000000001",
        RoleMask::from(RoleField::TRANSLATOR),
    ));
    mock.seed_page(page("page_source--00000000002", 2));
    mock.seed_page(page("page_source--00000000001", 1));

    seed_page_source_obj(
        &mock,
        "page_source--00000000002",
        2,
        true,
        (2, ImageExt::Png),
    );
    seed_page_source_obj(
        &mock,
        "page_source--00000000001",
        1,
        false,
        (1, ImageExt::Jpg),
    );

    let pages = list_infos(
        (&mock, &mock),
        page_source_token("user--00000000001"),
        ListPageSourceInfosInstr {
            chapter_id: "chapter--00000000001".into(),
        },
    )
    .await
    .unwrap();

    assert_eq!(pages.len(), 2);
    assert_eq!(pages.first().unwrap().id, "page_source--00000000001");
    assert_eq!(pages.first().unwrap().image_url, None);
    assert_eq!(pages.first().unwrap().image_optimized_url, None);
    assert_eq!(pages.first().unwrap().image_thumbnail_url, None);
    assert_eq!(pages.get(1).unwrap().id, "page_source--00000000002");
    assert_eq!(
        pages.get(1).unwrap().image_url.as_deref(),
        Some(
            "https://obj.test/page_source/chapter--00000000001/\
                page_source--00000000002/2.png"
        )
    );
    assert_eq!(
        pages.get(1).unwrap().image_optimized_url.as_deref(),
        Some(
            "https://obj.test/optimized/page_source/\
                chapter--00000000001/page_source--00000000002/2.png"
        )
    );
    assert_eq!(
        pages.get(1).unwrap().image_thumbnail_url.as_deref(),
        Some(
            "https://obj.test/thumbnail/page_source/\
                chapter--00000000001/page_source--00000000002/2.png"
        )
    );
}

#[tokio::test]
async fn list_infos_returns_complete_manifest_at_business_maximum() {
    let mock = Mock::new();

    seed_page_source_scope(&mock, MAX_CHAPTER_PAGE_SOURCE_COUNT);

    mock.seed_member(page_source_member(
        "user--00000000001",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    for index in (0..MAX_CHAPTER_PAGE_SOURCE_COUNT).rev() {
        mock.seed_page(page(&format!("page_source--{index:011}"), index));
    }

    let pages = list_infos(
        (&mock, &mock),
        page_source_token("user--00000000001"),
        ListPageSourceInfosInstr {
            chapter_id: "chapter--00000000001".into(),
        },
    )
    .await
    .unwrap();

    assert_eq!(pages.len(), MAX_CHAPTER_PAGE_SOURCE_COUNT);

    assert_eq!(pages.first().unwrap().id, "page_source--00000000000");

    assert_eq!(pages.last().unwrap().id, "page_source--00000000199");
}

#[tokio::test]
async fn list_infos_rejects_non_member_without_assignment() {
    let mock = Mock::new();

    seed_page_source_scope(&mock, 0);

    let error = list_infos(
        (&mock, &mock),
        page_source_token("user--00000000001"),
        ListPageSourceInfosInstr {
            chapter_id: "chapter--00000000001".into(),
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Perm);
}

#[tokio::test]
async fn list_infos_rejects_persisted_page_source_count_above_business_maximum()
{
    let mock = Mock::new();

    seed_page_source_scope(&mock, MAX_CHAPTER_PAGE_SOURCE_COUNT + 1);

    mock.seed_member(page_source_member(
        "user--00000000001",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    for index in 0..=MAX_CHAPTER_PAGE_SOURCE_COUNT {
        mock.seed_page(page(&format!("page_source--{index:011}"), index));
    }

    let error = list_infos(
        (&mock, &mock),
        page_source_token("user--00000000001"),
        ListPageSourceInfosInstr {
            chapter_id: "chapter--00000000001".into(),
        },
    )
    .await
    .unwrap_err();

    assert!(matches!(error, BaseError::Unrecoverable { .. }));
}

#[tokio::test]
async fn get_info_resolves_available_image_metadata_and_urls() {
    let mock = Mock::new();

    seed_page_source_scope(&mock, 1);

    mock.seed_member(page_source_member(
        "user--00000000001",
        RoleMask::from(RoleField::TRANSLATOR),
    ));
    mock.seed_page(page("page_source--00000000001", 0));

    seed_page_source_obj(
        &mock,
        "page_source--00000000001",
        7,
        true,
        (7, ImageExt::Png),
    );

    let found = get_info(
        (&mock, &mock),
        page_source_token("user--00000000001"),
        "page_source--00000000001".into(),
    )
    .await
    .unwrap();

    assert_eq!(found.id, "page_source--00000000001");
    assert_eq!(found.image_hash, Some(ImageHash::new([7; 32])));
    assert_eq!(found.ext, Some(ImageExt::Png));
    assert_eq!(
        found.image_url.as_deref(),
        Some(
            "https://obj.test/page_source/chapter--00000000001/\
                page_source--00000000001/7.png"
        )
    );
    assert_eq!(
        found.image_optimized_url.as_deref(),
        Some(
            "https://obj.test/optimized/page_source/\
                chapter--00000000001/page_source--00000000001/7.png"
        )
    );
    assert_eq!(
        found.image_thumbnail_url.as_deref(),
        Some(
            "https://obj.test/thumbnail/page_source/\
                chapter--00000000001/page_source--00000000001/7.png"
        )
    );
}

#[tokio::test]
async fn get_info_rejects_non_member_without_assignment() {
    let mock = Mock::new();

    seed_page_source_scope(&mock, 1);

    mock.seed_page(page("page_source--00000000001", 0));

    let error = get_info(
        (&mock, &mock),
        page_source_token("user--00000000001"),
        "page_source--00000000001".into(),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Perm);
}
