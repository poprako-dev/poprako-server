#![cfg(feature = "rdb_impl")]
#![allow(
    non_camel_case_types,
    reason = "Diesel table declarations preserve database column names"
)]

use poprako_obj_dept::objs_def;

diesel::table! {
    t_page_source_image (f_id) {
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

struct PageSourceImage;

objs_def! {
    PageSourceImage {
        table: t_page_source_image,
        topic: "page_source_image",
    },
}

#[test]
fn expands_direct_typed_object() {
    let _: PageSourceImage = PageSourceImage;

    assert_eq!(page_source_image_rdb_impl::TOPIC, "page_source_image");
}
