use std::collections::HashMap;
use std::marker::PhantomData;

use poprako_orchestra::Oper;

use crate::key::ObjKey;
use crate::model::meta::ObjMeta;
use crate::model::slot::{ObjSlot, ObjSlotSpec};
use crate::model::url::ObjUrls;

/// Reads current object metadata for a collection of business objects.
#[derive(Oper)]
#[oper(output = HashMap<String, ObjMeta>)]
pub struct ListObjMetas<'a, B> {
    //
    /// Stable business-object identifiers.
    pub ids: &'a [String],
    /// Compile-time object marker selected for this operation.
    #[doc(hidden)]
    _m: PhantomData<fn() -> B>,
}

impl<'a, B> ListObjMetas<'a, B> {
    /// Creates a metadata lookup for the supplied business-object identifiers.
    #[must_use]
    pub const fn new(ids: &'a [String]) -> Self {
        //
        Self {
            ids,
            _m: PhantomData,
        }
    }
}

/// Generates read URLs for the supplied metadata versions.
#[derive(Oper)]
#[oper(output = HashMap<String, ObjUrls>)]
pub struct GenObjUrls<'a, B> {
    //
    /// Metadata versions whose physical keys will be resolved.
    pub metas: &'a HashMap<String, ObjMeta>,
    /// Compile-time object marker selected for this operation.
    #[doc(hidden)]
    _m: PhantomData<fn() -> B>,
}

impl<'a, B> GenObjUrls<'a, B> {
    /// Creates a read-URL request for the supplied object metadata.
    #[must_use]
    pub const fn new(metas: &'a HashMap<String, ObjMeta>) -> Self {
        //
        Self {
            metas,
            _m: PhantomData,
        }
    }
}

/// Generates a new generation and its locally signed write capability.
#[derive(Oper)]
#[oper(output = ObjSlot)]
pub struct GenObjSlot<'a, B> {
    //
    /// Business-planned object and write requirements.
    pub spec: &'a ObjSlotSpec<'a>,
    /// Compile-time object marker selected for this operation.
    #[doc(hidden)]
    _m: PhantomData<fn() -> B>,
}

impl<'a, B> GenObjSlot<'a, B> {
    /// Creates a reservation for one business object.
    #[must_use]
    pub const fn new(spec: &'a ObjSlotSpec<'a>) -> Self {
        //
        Self {
            spec,
            _m: PhantomData,
        }
    }
}

/// Generates new generations and locally signed write capabilities in bulk.
#[derive(Oper)]
#[oper(output = HashMap<String, ObjSlot>)]
pub struct GenObjSlots<'a, B> {
    //
    /// Business-planned objects and their write requirements.
    pub specs: &'a [ObjSlotSpec<'a>],
    /// Compile-time object marker selected for this operation.
    #[doc(hidden)]
    _m: PhantomData<fn() -> B>,
}

impl<'a, B> GenObjSlots<'a, B> {
    /// Creates a bulk reservation for the supplied business objects.
    #[must_use]
    pub const fn new(specs: &'a [ObjSlotSpec<'a>]) -> Self {
        //
        Self {
            specs,
            _m: PhantomData,
        }
    }
}

/// Optimistically marks one exact current object generation as uploaded.
#[derive(Oper)]
#[oper(output = bool)]
pub struct MarkObjUploaded<'a, B> {
    //
    /// Exact logical object generation declared uploaded by the client.
    pub key: &'a ObjKey,
    /// Compile-time object marker selected for this operation.
    #[doc(hidden)]
    _m: PhantomData<fn() -> B>,
}

impl<'a, B> MarkObjUploaded<'a, B> {
    /// Creates an upload declaration for one exact object generation.
    #[must_use]
    pub const fn new(key: &'a ObjKey) -> Self {
        //
        Self {
            key,
            _m: PhantomData,
        }
    }
}

/// Clears current objects while their owning business entities remain active.
#[derive(Oper)]
#[oper(output = ())]
pub struct ClearObjs<'a, B> {
    //
    /// Business-object identifiers whose current files are cleared.
    pub ids: &'a [String],
    /// Compile-time object marker selected for this operation.
    #[doc(hidden)]
    _m: PhantomData<fn() -> B>,
}

impl<'a, B> ClearObjs<'a, B> {
    /// Creates a request to clear current files for active business entities.
    #[must_use]
    pub const fn new(ids: &'a [String]) -> Self {
        //
        Self {
            ids,
            _m: PhantomData,
        }
    }
}

/// Deletes objects whose owning business entities have ended their lifecycle.
///
/// Each supplied business identifier must never be reused for this object kind.
#[derive(Oper)]
#[oper(output = ())]
pub struct DeleteObjs<'a, B> {
    //
    /// Permanently retired business-object identifiers.
    pub ids: &'a [String],
    /// Compile-time object marker selected for this operation.
    #[doc(hidden)]
    _m: PhantomData<fn() -> B>,
}

impl<'a, B> DeleteObjs<'a, B> {
    /// Creates a request to delete objects for ended business entities.
    #[must_use]
    pub const fn new(ids: &'a [String]) -> Self {
        //
        Self {
            ids,
            _m: PhantomData,
        }
    }
}
