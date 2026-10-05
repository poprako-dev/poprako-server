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

use crate::key::KeyMap;
use crate::oper::{
    ClearObjs, DeleteObjs, GenObjSlot, GenObjSlots, GenObjUrls, ListObjMetas,
    MarkObjUploaded,
};
use crate::rest::ObjDeptError;
/// UUID execution credentials shared with generated adapters.
pub use uuid::Uuid;

#[cfg(feature = "rdb_impl")]
pub use poprako_obj_dept_macro::{
    expand_obj_dept_items, impl_obj_dept, objs_def, rdb_obj_dept_prom,
};
#[cfg(feature = "rdb_impl")]
pub use poprako_rdb_core::{RdbContext, RdbCore};

extern crate self as poprako_obj_dept;

/// Read-only object operations for one compile-time marker.
#[drive(
    context = C,
    error = ObjDeptError,
    run(
        for<'a> ListObjMetas<'a, K>,
        for<'a> GenObjUrls<'a, K>,
    ),
    step(for<'a> ListObjMetas<'a, K>),
)]
pub trait ObjDeptView<K, C>
where
    K: KeyMap,
{
}

/// Writable object operations for one compile-time marker.
#[drive(
    context = C,
    error = ObjDeptError,
    run(
        for<'a> ListObjMetas<'a, K>,
        for<'a> GenObjUrls<'a, K>,
        for<'a> MarkObjUploaded<'a, K>,
    ),
    step(
        for<'a> MarkObjUploaded<'a, K>,
        for<'a> ListObjMetas<'a, K>,
        for<'a> GenObjSlot<'a, K>,
        for<'a> GenObjSlots<'a, K>,
        for<'a> ClearObjs<'a, K>,
        for<'a> DeleteObjs<'a, K>,
    ),
)]
pub trait ObjDept<K, C>
where
    K: KeyMap,
{
}
