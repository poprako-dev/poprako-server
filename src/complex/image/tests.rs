use super::ImageComplex;

use crate::config::image::ImageConfig;
use crate::result::BaseError;
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

        let failure = ImageComplex::ensure_byte_length(
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
fn missing_byte_length_uses_the_runtime_page_limit_in_its_message() {
    //
    let failure = ImageComplex::invalid_byte_length_rejection(
        &IMAGE_CONFIG,
        0,
        ImageKind::PageImage,
    );

    let BaseError::Expected { message, .. } = failure else {
        panic!("missing image length must remain client-correctable");
    };

    assert!(message.contains('4'));

    assert!(message.contains("MiB"));

    assert!(!message.contains("20 MiB"));
}
