#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

use crate::complex::image as image_complex;
use crate::value::artwork::ArtworkExt;
use crate::value::business_id::{
    ChapterId, ComicId, PageArtworkId, PageSourceId, TeamId, UserId,
};

use crate::config::image::ImageConfig;
use crate::result::BaseError;
use crate::value::image::ImageKind;

const IMAGE_CONFIG: ImageConfig = ImageConfig {
    user_avatar_limit_mib: 1,
    team_avatar_limit_mib: 2,
    comic_cover_limit_mib: 3,
    page_source_image_limit_mib: 4,
    page_artwork_image_limit_mib: 5,
};

#[test]
#[expect(
    clippy::panic,
    reason = "This test fails explicitly when an expected fixture variant or assertion is violated"
)]
fn byte_length_uses_each_kind_specific_configured_mib_limit() {
    //
    let cases = [
        (ImageKind::UserAvatar, 1024 * 1024),
        (ImageKind::TeamAvatar, 2 * 1024 * 1024),
        (ImageKind::ComicCover, 3 * 1024 * 1024),
        (ImageKind::PageSourceImage, 4 * 1024 * 1024),
        (ImageKind::PageArtworkImage, 5 * 1024 * 1024),
    ];

    for (image_kind, byte_limit) in cases {
        //
        assert!(
            image_complex::ensure_byte_length(
                &IMAGE_CONFIG,
                byte_limit,
                image_kind,
            )
            .is_ok(),
        );

        let failure = image_complex::ensure_byte_length(
            &IMAGE_CONFIG,
            byte_limit + 1,
            image_kind,
        )
        .err()
        .unwrap();

        let BaseError::Expected { message, .. } = failure else {
            panic!("invalid image length must remain client-correctable");
        };

        let configured_mib = IMAGE_CONFIG.limit_for(image_kind);

        assert!(message.contains(&configured_mib.to_string()));

        assert!(message.contains("MiB"));
    }
}

#[test]
#[expect(
    clippy::panic,
    reason = "This test fails explicitly when an expected fixture variant or assertion is violated"
)]
fn missing_byte_length_uses_the_runtime_page_source_limit_in_its_message() {
    //
    let failure = image_complex::invalid_byte_length_rejection(
        &IMAGE_CONFIG,
        0,
        ImageKind::PageSourceImage,
    );

    let BaseError::Expected { message, .. } = failure else {
        panic!("missing image length must remain client-correctable");
    };

    assert!(message.contains('4'));

    assert!(message.contains("MiB"));

    assert!(!message.contains("20 MiB"));
}

// physical_keys(ObjKind)(positive): all six one-way mappings preserve the established storage layout.
#[test]
fn physical_keys_preserve_canonical_generation_paths() {
    use crate::part::obj_dept::{
        ChapterArtwork, ComicCover, PageArtworkImage, PageSourceImage,
        TeamAvatar, UserAvatar,
    };
    use crate::value::artwork::ChapterArtworkKey;
    use crate::value::image::{
        ComicCoverKey, ImageExt, PageSourceImageKey, TeamAvatarKey,
        UserAvatarKey,
    };
    use crate::value::page_artwork::PageArtworkImageKey;

    let source = PageSourceImageKey {
        chapter_id: ChapterId::new("chapter--00000000001").unwrap(),
        page_source_id: PageSourceId::new("page_source--00000000001").unwrap(),
        ext: ImageExt::Png,
    };

    let artwork = PageArtworkImageKey {
        chapter_id: source.chapter_id.clone(),
        page_artwork_id: PageArtworkId::new("page_artwork--00000000001")
            .unwrap(),
        ext: ImageExt::Png,
    };

    let chapter = ChapterArtworkKey {
        chapter_id: source.chapter_id.clone(),
        ext: ArtworkExt::new("zip").unwrap(),
    };

    let comic = ComicCoverKey {
        comic_id: ComicId::new("comic--00000000001").unwrap(),
        ext: ImageExt::Png,
    };

    let team = TeamAvatarKey {
        team_id: TeamId::new("team--00000000001").unwrap(),
        ext: ImageExt::Png,
    };

    let user = UserAvatarKey {
        user_id: UserId::new("user--00000000001").unwrap(),
        ext: ImageExt::Png,
    };

    assert_key::<PageSourceImage>(
        &source,
        "page_source/chapter--00000000001/page_source--00000000001/7.png",
    );

    assert_key::<PageArtworkImage>(
        &artwork,
        "page_artwork/chapter--00000000001/page_artwork--00000000001/7.png",
    );

    assert_key::<ChapterArtwork>(
        &chapter,
        "chapter_artwork/chapter--00000000001/7.zip",
    );

    assert_key::<ComicCover>(&comic, "comic_cover/comic--00000000001/7.png");

    assert_key::<TeamAvatar>(&team, "team_avatar/team--00000000001/7.png");

    assert_key::<UserAvatar>(&user, "user_avatar/user--00000000001/7.png");
}

fn assert_key<M: poprako_obj_dept::key::ObjKind>(dom: &M::Dom, expected: &str) {
    assert_eq!(M::forward(dom, 7).as_str(), expected);
}
