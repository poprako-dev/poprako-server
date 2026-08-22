use super::ImageComplex;

use crate::config::ImageConfig;
use crate::value::image::ImageKind;

const IMAGE_CONFIG: ImageConfig = ImageConfig {
    user_avatar_limit: 1,
    team_avatar_limit: 2,
    comic_cover_limit: 3,
    page_image_limit: 4,
};

#[test]
fn byte_length_uses_each_kind_specific_configured_mib_limit() {
    //
    let cases = [
        (ImageKind::UserAvatar, 1024 * 1024),
        (ImageKind::TeamAvatar, 2 * 1024 * 1024),
        (ImageKind::ComicCover, 3 * 1024 * 1024),
        (ImageKind::PageImage, 4 * 1024 * 1024),
    ];

    for (image_kind, byte_limit) in cases {
        //
        assert!(
            ImageComplex::ensure_byte_length(
                &IMAGE_CONFIG,
                byte_limit,
                image_kind,
            )
            .is_ok(),
        );

        assert!(
            ImageComplex::ensure_byte_length(
                &IMAGE_CONFIG,
                byte_limit + 1,
                image_kind,
            )
            .is_err(),
        );
    }
}
