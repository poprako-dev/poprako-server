#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately on violated invariants"
)]

use super::{alloc, config, mark, page, seed, token};

use crate::data::instr::page_artwork::AllocPageArtworkImageInstr;
use crate::part_impl::repo::mock_impl::MockContext;
use crate::usecase::page_artwork as page_artwork_usecase;
use crate::usecase::page_artwork::image as page_artwork_image_usecase;
use crate::value::image::{ImageExt, ImageHash};
use crate::value::role::RoleField;

// list_infos(list_infos)(positive): available composites expose raw and thumbnail URLs; unavailable generations expose no read URLs.
#[tokio::test]
async fn composite_list_exposes_only_raw_and_thumbnail_images() {
    let mock = seed(RoleField::REVIEWER);

    let allocated = alloc(&mock, vec![page(None, 1), page(None, 2)])
        .await
        .unwrap();

    let uploaded_page = allocated.pages.first().unwrap();

    mark(
        &mock,
        &uploaded_page.page_artwork_id,
        uploaded_page.image_ver,
    )
    .await
    .unwrap();

    let page_artwork_info_views =
        page_artwork_usecase::list_infos::<MockContext, _, _>(
            (&mock, &mock),
            token("user-1"),
            "chapter-1".into(),
        )
        .await
        .unwrap();

    let image_url = page_artwork_info_views
        .first()
        .unwrap()
        .image_url
        .as_ref()
        .unwrap();

    let image_thumbnail_url = page_artwork_info_views
        .first()
        .unwrap()
        .image_thumbnail_url
        .as_ref()
        .unwrap();

    assert!(image_url.starts_with("https://obj.test/page_artwork/"));

    assert!(image_thumbnail_url.starts_with("https://obj.test/thumbnail/"));

    let payload = serde_json::to_value(&page_artwork_info_views).unwrap();

    let pages = payload.as_array().unwrap();

    assert!(
        pages
            .iter()
            .all(|page| page.get("image_optimized_url").is_none())
    );

    let unavailable_page = pages.get(1).unwrap();

    assert!(unavailable_page.get("image_url").is_none());

    assert!(unavailable_page.get("image_thumbnail_url").is_none());
}

// alloc_image(alloc_image)(positive): replacing one composite preserves all sibling metadata, including update timestamps and positions.
#[tokio::test]
async fn single_image_allocation_preserves_sibling_metadata() {
    let mock = seed(RoleField::REVIEWER);

    let allocated = alloc(&mock, vec![page(None, 1), page(None, 2)])
        .await
        .unwrap();

    let page_artwork_id = &allocated.pages.first().unwrap().page_artwork_id;

    let before = mock.snapshot().page_artworks;

    let alloc_page_artwork_image_instr = AllocPageArtworkImageInstr {
        raw_ident: Some("renamed.psd".into()),
        image_hash: ImageHash::new([3; 32]),
        new_byte_len: 100,
        ext: ImageExt::Png,
    };

    page_artwork_image_usecase::alloc_image(
        (&mock, &mock, &mock, &config()),
        token("user-1"),
        page_artwork_id.clone(),
        alloc_page_artwork_image_instr,
    )
    .await
    .unwrap();

    let after = mock.snapshot().page_artworks;

    let sibling_before = before
        .iter()
        .find(|info| &info.id != page_artwork_id)
        .unwrap();

    let sibling_after = after
        .iter()
        .find(|info| info.id == sibling_before.id)
        .unwrap();

    assert_eq!(sibling_after, sibling_before);

    let updated = after
        .iter()
        .find(|info| &info.id == page_artwork_id)
        .unwrap();

    assert_eq!(updated.raw_ident.as_deref(), Some("renamed.psd"));

    assert_eq!(updated.index, 0);

    assert_eq!(updated.chapter_id, "chapter-1");
}
