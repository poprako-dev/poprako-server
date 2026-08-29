use proc_macro2::TokenStream;
use quote::quote;

use crate::object;

// Returns the declaration error text, or an empty string when expansion succeeds.
fn expand_err(input: TokenStream) -> String {
    match object::expand(input) {
        Ok(_) => String::new(),
        Err(err) => err.to_string(),
    }
}

#[test]
fn object_manifest_rejects_every_duplicate_identity() {
    //
    let duplicate_marker = expand_err(quote! {
        PageImage {
            table: t_page_image,
            topic: "page_image",
            namespace: "page_image",
        },
        PageImage {
            table: t_page_image_2,
            topic: "page_image_2",
            namespace: "page_image_2",
        },
    });

    let duplicate_table = expand_err(quote! {
        PageImage {
            table: t_page_image,
            topic: "page_image",
            namespace: "page_image",
        },
        ComicCover {
            table: t_page_image,
            topic: "comic_cover",
            namespace: "comic_cover",
        },
    });

    let duplicate_topic = expand_err(quote! {
        PageImage {
            table: t_page_image,
            topic: "page_image",
            namespace: "page_image",
        },
        ComicCover {
            table: t_comic_cover,
            topic: "page_image",
            namespace: "comic_cover",
        },
    });

    let duplicate_namespace = expand_err(quote! {
        PageImage {
            table: t_page_image,
            topic: "page_image",
            namespace: "page_image",
        },
        ComicCover {
            table: t_comic_cover,
            topic: "comic_cover",
            namespace: "page_image",
        },
    });

    assert_eq!(duplicate_marker, "duplicate object marker");

    assert_eq!(duplicate_table, "duplicate object table");

    assert_eq!(duplicate_topic, "duplicate object topic");

    assert_eq!(duplicate_namespace, "duplicate object namespace");
}
