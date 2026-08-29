use std::marker::PhantomData;

use poprako_orchestra::Oper;
use url::Url;

use crate::model::meta::ObjMeta;
use crate::model::slot::{ObjSlot, ObjSlotSpec};

/// Reads the latest RDB metadata for one business object.
#[derive(Oper)]
#[oper(output = Option<ObjMeta>)]
pub struct GetObjMeta<'a, B> {
    /// Stable business-object identifier.
    pub id: &'a str,
    /// Compile-time object marker selected for this operation.
    #[doc(hidden)]
    pub _m: PhantomData<fn() -> B>,
}

/// Generates a read URL for the verified current object.
#[derive(Oper)]
#[oper(output = Option<Url>)]
pub struct GenObjUrl<'a, B> {
    /// Stable business-object identifier.
    pub id: &'a str,
    /// Compile-time object marker selected for this operation.
    #[doc(hidden)]
    pub _m: PhantomData<fn() -> B>,
}

/// Generates a new generation and its locally signed write capability.
#[derive(Oper)]
#[oper(output = ObjSlot)]
pub struct GenObjSlot<'a, B> {
    /// Business-planned object and write requirements.
    pub spec: &'a ObjSlotSpec<'a>,
    /// Compile-time object marker selected for this operation.
    #[doc(hidden)]
    pub _m: PhantomData<fn() -> B>,
}

/// Reliably retires current objects inside the caller-owned transaction.
#[derive(Oper)]
#[oper(output = ())]
pub enum DelObjs<'a, B> {
    /// Defers physical deletion and retains each identifier's version watermark.
    Detach {
        /// Business-object identifiers to detach.
        ids: &'a [String],
        /// Compile-time object marker selected for this operation.
        #[doc(hidden)]
        _m: PhantomData<fn() -> B>,
    },

    /// Defers physical deletion and removes each object row.
    Remove {
        /// Business-object identifiers whose rows are removed.
        ids: &'a [String],
        /// Compile-time object marker selected for this operation.
        #[doc(hidden)]
        _m: PhantomData<fn() -> B>,
    },
}
