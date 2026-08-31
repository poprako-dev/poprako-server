#![cfg(feature = "rdb_impl")]
#![allow(non_camel_case_types)]

use poprako_obj_dept::objs_def;

diesel::table! {
    t_page_image (f_id) {
        f_id -> Text,
        f_version -> BigInt,
        f_key -> Nullable<Text>,
        f_is_uploaded -> Nullable<Bool>,
        f_hash -> Nullable<Binary>,
        f_ext -> Nullable<Text>,
        f_created_at -> Timestamptz,
        f_updated_at -> Timestamptz,
    }
}

struct PageImage;

objs_def! {
    PageImage {
        table: t_page_image,
        topic: "page_image",
        url_profile: ImageThumbnail,
    },
}

#[test]
fn expands_direct_typed_object() {
    let _marker = PageImage;

    assert_eq!(page_image_rdb_impl::TOPIC, "page_image");

    assert_eq!(
        page_image_rdb_impl::URL_PROFILE,
        poprako_obj_dept::pool::ObjUrlProfile::ImageThumbnail,
    );
}
