use super::super::view::page_info_views;

use time::OffsetDateTime;

use poprako_obj_dept::key::ObjKey;
use poprako_obj_dept::model::meta::ObjMeta;

use crate::model::read::proj::page::PageInfo;
use crate::part_impl::repo::mock_impl::{Mock, MockObjRecord};
use crate::value::image::{ImageExt, ImageHash};

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
            version,
            image: format!("page/chapter_test/{}-{}.{}", id, version, ext),
        },
        is_available: uploaded,
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
            version: uploaded_meta.key.version,
            meta: Some(uploaded_meta),
        },
    );

    page_images.insert(
        "page-pending".into(),
        MockObjRecord {
            version: pending_meta.key.version,
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
        uploaded_view.image_thumbnail_url.as_deref(),
        Some(
            "https://obj.test/thumbnail/page/chapter_test/page-uploaded-7.png"
        )
    );

    assert_eq!(uploaded_view.image_hash, Some(ImageHash::new([7; 32])));

    assert_eq!(uploaded_view.ext, Some(ImageExt::Png));

    let pending_view = &page_views[1];

    assert_eq!(pending_view.image_url, None);

    assert_eq!(pending_view.image_thumbnail_url, None);

    assert_eq!(pending_view.image_hash, Some(ImageHash::new([4; 32])));

    assert_eq!(pending_view.ext, Some(ImageExt::Jpg));

    let missing_view = &page_views[2];

    assert_eq!(missing_view.image_url, None);

    assert_eq!(missing_view.image_thumbnail_url, None);

    assert_eq!(missing_view.image_hash, None);

    assert_eq!(missing_view.ext, None);
}
