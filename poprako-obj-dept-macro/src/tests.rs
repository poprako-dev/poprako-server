use proc_macro2::TokenStream;
use quote::quote;

use crate::{impl_obj_dept, object};

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
            url_profile: ImageThumbnail,
        },
        PageImage {
            table: t_page_image_2,
            topic: "page_image_2",
            namespace: "page_image_2",
            url_profile: ImageThumbnail,
        },
    });

    let duplicate_table = expand_err(quote! {
        PageImage {
            table: t_page_image,
            topic: "page_image",
            namespace: "page_image",
            url_profile: ImageThumbnail,
        },
        ComicCover {
            table: t_page_image,
            topic: "comic_cover",
            namespace: "comic_cover",
            url_profile: ImageThumbnail,
        },
    });

    let duplicate_topic = expand_err(quote! {
        PageImage {
            table: t_page_image,
            topic: "page_image",
            namespace: "page_image",
            url_profile: ImageThumbnail,
        },
        ComicCover {
            table: t_comic_cover,
            topic: "page_image",
            namespace: "comic_cover",
            url_profile: ImageThumbnail,
        },
    });

    let duplicate_namespace = expand_err(quote! {
        PageImage {
            table: t_page_image,
            topic: "page_image",
            namespace: "page_image",
            url_profile: ImageThumbnail,
        },
        ComicCover {
            table: t_comic_cover,
            topic: "comic_cover",
            namespace: "page_image",
            url_profile: ImageThumbnail,
        },
    });

    assert_eq!(duplicate_marker, "duplicate object marker");

    assert_eq!(duplicate_table, "duplicate object table");

    assert_eq!(duplicate_topic, "duplicate object topic");

    assert_eq!(duplicate_namespace, "duplicate object namespace");
}

#[test]
fn generated_names_are_readable_and_read_views_cover_every_marker()
-> syn::Result<()> {
    let manifest = object::expand(quote! {
        PageImage {
            table: t_page_image,
            topic: "page_image",
            namespace: "page_image",
            url_profile: ImageThumbnail,
        },
        FontFile {
            table: t_font_file,
            topic: "font_file",
            namespace: "font_file",
            url_profile: OriginOnly,
        },
    })?
    .to_string();
    let implementations = impl_obj_dept::expand_items(quote! {
        dept: NormObjDept,
        view: NormObjView;
        (PageImage, page_image_rdb_impl, "page_image", "page_image", ImageThumbnail),
        (FontFile, font_file_rdb_impl, "font_file", "font_file", OriginOnly),
    })?
    .to_string();

    assert!(manifest.contains("page_image_rdb_impl"));

    assert!(manifest.contains("font_file_rdb_impl"));

    assert!(manifest.contains("URL_PROFILE"));

    assert!(manifest.contains("ImageThumbnail"));

    assert!(manifest.contains("OriginOnly"));

    assert!(manifest.contains("for_each_obj"));

    assert!(!manifest.contains("__"));

    assert!(implementations.contains("NormObjView"));

    assert!(implementations.contains("PageImage"));

    assert!(implementations.contains("FontFile"));

    assert!(implementations.contains("URL_PROFILE"));

    assert!(!implementations.contains("__"));

    assert_eq!(implementations.matches("ensure_anchors").count(), 2);

    assert!(!implementations.contains("advisory"));

    assert!(manifest.contains("on_conflict"));

    assert!(manifest.contains("do_update"));

    assert!(manifest.contains("as_returning"));

    assert!(manifest.contains("load_for_presence_reconciliation"));

    assert!(manifest.contains("f_updated_at . eq (revision)"));

    assert!(implementations.contains("rows . len () != ids . len ()"));

    Ok(())
}
