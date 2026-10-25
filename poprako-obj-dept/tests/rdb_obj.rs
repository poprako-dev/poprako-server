#![cfg(feature = "rdb_impl")]
#![allow(
    non_camel_case_types,
    reason = "Diesel table declarations preserve database column names"
)]

use poprako_obj_dept::{RdbCore, impl_obj_dept, objs_def};

diesel::table! {
    t_page_source_image (f_id) {
        f_id -> Text,
        f_version -> BigInt,
        f_domain -> Nullable<Jsonb>,
        f_is_uploaded -> Nullable<Bool>,
        f_hash -> Nullable<Binary>,
        f_ext -> Nullable<Text>,
        f_created_at -> Timestamptz,
        f_updated_at -> Timestamptz,
    }
}

#[path = "../src/tests/test_support.rs"]
mod test_support;

use test_support::PageSourceImage;

objs_def! {
    PageSourceImage {
        table: t_page_source_image,
        topic: "page_source_image",
    },
}

/// Generic composition used to type-check the complete generated interface.
pub struct TestDept<P, M> {
    core: RdbCore,
    pool: P,
    prom: M,
}

impl<P, M> TestDept<P, M> {
    const fn core(&self) -> &RdbCore {
        &self.core
    }

    const fn pool(&self) -> &P {
        &self.pool
    }

    const fn prom(&self) -> &M {
        &self.prom
    }
}

/// Read projection for associated scalar types with no inherent string methods.
pub struct TestView<P> {
    core: RdbCore,
    pool: P,
}

impl<P> TestView<P> {
    const fn core(&self) -> &RdbCore {
        &self.core
    }

    const fn pool(&self) -> &P {
        &self.pool
    }
}

impl_obj_dept! {
    dept: TestDept,
    view: TestView,
}

#[test]
fn expands_direct_typed_object() {
    let _: PageSourceImage = PageSourceImage;

    assert_eq!(page_source_image_rdb_impl::TOPIC, "page_source_image");
}
