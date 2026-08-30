#![deny(clippy::correctness)]
#![deny(clippy::suspicious)]
#![deny(clippy::complexity)]
#![deny(clippy::perf)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::dbg_macro)]
#![deny(clippy::print_stdout)]
#![deny(clippy::print_stderr)]
#![deny(clippy::exit)]
#![deny(clippy::indexing_slicing)]
#![deny(clippy::string_slice)]
#![deny(clippy::mod_module_files)]
#![warn(clippy::style)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(clippy::future_not_send)]
#![allow(clippy::unnecessary_wraps)]
#![allow(clippy::uninlined_format_args)]

/// Generic actor lifecycle.
pub mod actor;
/// Immutable logical object identities.
pub mod key;
/// Object metadata, upload capability, and task values.
pub mod model;
/// Orchestra operation descriptors for objects.
pub mod oper;
/// Physical object-storage contract.
pub mod pool;
/// Durable Check/Delete contract.
pub mod prom;
/// Rest and error types for object operations.
pub mod rest;

#[cfg(feature = "rdb_impl")]
/// Diesel-backed object contract.
pub mod rdb_impl;

#[cfg(test)]
mod tests;

use poprako_orchestra::drive;

use crate::oper::{
    GenObjSlot, GenObjSlots, GenObjUrls, ListObjMetas, MarkObjUploaded,
    RetireObjs,
};
use crate::rest::ObjDeptError;

pub use crate::model::mark::MarkObjUploadedOutcome;

#[cfg(feature = "rdb_impl")]
pub use poprako_obj_dept_macro::{
    expand_obj_dept_items, impl_obj_dept, objs_def, rdb_obj_prom,
};
#[cfg(feature = "rdb_impl")]
pub use poprako_rdb_core::{RdbContext, RdbCore};

extern crate self as poprako_obj_dept;

/// Read-only object operations for one compile-time marker.
#[drive(
    context = C,
    error = ObjDeptError,
    run(
        for<'a> ListObjMetas<'a, B>,
        for<'a> GenObjUrls<'a, B>,
    ),
    step(for<'a> ListObjMetas<'a, B>),
)]
pub trait ObjDeptView<B, C> {}

/// Writable object operations for one compile-time marker.
#[drive(
    context = C,
    error = ObjDeptError,
    run(
        for<'a> ListObjMetas<'a, B>,
        for<'a> GenObjUrls<'a, B>,
        for<'a> MarkObjUploaded<'a, B>,
    ),
    step(
        for<'a> ListObjMetas<'a, B>,
        for<'a> GenObjSlot<'a, B>,
        for<'a> GenObjSlots<'a, B>,
        for<'a> RetireObjs<'a, B>,
    ),
)]
pub trait ObjDept<B, C> {}

/// Constructs an `ObjDept` operation while hiding its marker field.
#[macro_export]
// Expands an operation while supplying its marker field.
macro_rules! obj_inst {
    (ListObjMetas<$obj:ident> { ids: $ids:expr $(,)? }) => {
        ::poprako_obj_dept::oper::ListObjMetas::<$obj> {
            ids: $ids,
            _m: ::core::marker::PhantomData,
        }
    };
    (GenObjUrls<$obj:ident> { metas: $metas:expr $(,)? }) => {
        ::poprako_obj_dept::oper::GenObjUrls::<$obj> {
            metas: $metas,
            _m: ::core::marker::PhantomData,
        }
    };
    (GenObjSlot<$obj:ident> { spec: $spec:expr $(,)? }) => {
        ::poprako_obj_dept::oper::GenObjSlot::<$obj> {
            spec: $spec,
            _m: ::core::marker::PhantomData,
        }
    };
    (GenObjSlots<$obj:ident> { specs: $specs:expr $(,)? }) => {
        ::poprako_obj_dept::oper::GenObjSlots::<$obj> {
            specs: $specs,
            _m: ::core::marker::PhantomData,
        }
    };
    (MarkObjUploaded<$obj:ident> { key: $key:expr $(,)? }) => {
        ::poprako_obj_dept::oper::MarkObjUploaded::<$obj> {
            key: $key,
            _m: ::core::marker::PhantomData,
        }
    };
    (RetireObjs<$obj:ident>::PreserveWatermarks { ids: $ids:expr $(,)? }) => {{
        let oper: ::poprako_obj_dept::oper::RetireObjs<'_, $obj> =
            ::poprako_obj_dept::oper::RetireObjs::PreserveWatermarks {
                ids: $ids,
                _m: ::core::marker::PhantomData,
            };
        oper
    }};
    (RetireObjs<$obj:ident>::RemoveRows { ids: $ids:expr $(,)? }) => {{
        let oper: ::poprako_obj_dept::oper::RetireObjs<'_, $obj> =
            ::poprako_obj_dept::oper::RetireObjs::RemoveRows {
                ids: $ids,
                _m: ::core::marker::PhantomData,
            };
        oper
    }};
}
