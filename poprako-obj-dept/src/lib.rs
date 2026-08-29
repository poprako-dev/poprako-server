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
/// Generic actor lifecycle and state rules.
pub mod actor;
#[cfg(feature = "rdb_impl")]
/// Diesel-backed object contract.
pub mod rdb_impl;

#[cfg(test)]
mod tests;

use poprako_orchestra::drive;

use crate::oper::{DelObjs, GenObjSlot, GenObjUrl, GetObjMeta};
use crate::rest::ObjDeptError;

#[cfg(feature = "rdb_impl")]
pub use poprako_obj_dept_macro::{
    impl_obj_dept, impl_obj_dept_items as __impl_obj_dept_items, objs_def,
    rdb_obj_prom,
};
#[cfg(feature = "rdb_impl")]
pub use poprako_rdb_core::{RdbContext, RdbCore};

extern crate self as poprako_obj_dept;

/// Reliable object operations for one compile-time marker.
#[drive(
    context = C,
    error = ObjDeptError,
    run(
        for<'a> GetObjMeta<'a, B>,
        for<'a> GenObjUrl<'a, B>,
    ),
    step(
        for<'a> GetObjMeta<'a, B>,
        for<'a> GenObjSlot<'a, B>,
        for<'a> DelObjs<'a, B>,
    ),
)]
pub trait ObjDept<B, C> {}

// Constructs an ObjDept operation while hiding its marker field.
/// Constructs an `ObjDept` operation while hiding its marker field.
#[macro_export]
// Expands an operation while supplying its marker field.
macro_rules! obj_inst {
    (GetObjMeta<$obj:ident> { id: $id:expr $(,)? }) => {
        ::poprako_obj_dept::oper::GetObjMeta::<$obj> {
            id: $id,
            _m: ::core::marker::PhantomData,
        }
    };
    (GenObjUrl<$obj:ident> { id: $id:expr $(,)? }) => {
        ::poprako_obj_dept::oper::GenObjUrl::<$obj> {
            id: $id,
            _m: ::core::marker::PhantomData,
        }
    };
    (GenObjSlot<$obj:ident> { spec: $spec:expr $(,)? }) => {
        ::poprako_obj_dept::oper::GenObjSlot::<$obj> {
            spec: $spec,
            _m: ::core::marker::PhantomData,
        }
    };
    (DelObjs<$obj:ident>::Detach { ids: $ids:expr $(,)? }) => {{
        let oper: ::poprako_obj_dept::oper::DelObjs<'_, $obj> =
            ::poprako_obj_dept::oper::DelObjs::Detach {
                ids: $ids,
                _m: ::core::marker::PhantomData,
            };
        oper
    }};
    (DelObjs<$obj:ident>::Remove { ids: $ids:expr $(,)? }) => {{
        let oper: ::poprako_obj_dept::oper::DelObjs<'_, $obj> =
            ::poprako_obj_dept::oper::DelObjs::Remove {
                ids: $ids,
                _m: ::core::marker::PhantomData,
            };
        oper
    }};
}
