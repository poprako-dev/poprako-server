use super::ImageComplex;

use crate::config::ImageConfig;
use crate::value::image::ImageKind;

const IMAGE_CONFIG: ImageConfig = ImageConfig {
    user_avatar_limit: 101,
    team_avatar_limit: 102,
    comic_cover_limit: 103,
    page_image_limit: 104,
};

#[test]
fn byte_length_uses_each_kind_specific_configured_limit() {
    //
    let cases = [
        (ImageKind::UserAvatar, 101),
        (ImageKind::TeamAvatar, 102),
        (ImageKind::ComicCover, 103),
        (ImageKind::PageImage, 104),
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
