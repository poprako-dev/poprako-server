use std::collections::HashMap;
use std::marker::PhantomData;

use poprako_orchestra::Oper;

use crate::key::{KeyMap, ObjGen};
use crate::model::meta::ObjMeta;
use crate::model::slot::{ObjSlot, ObjSlotSpec};
use crate::model::url::{ObjUrlSpec, ObjUrls};

/// Reads current object metadata for a collection of business objects.
#[derive(Oper)]
#[oper(output = HashMap<String, ObjMeta>)]
pub struct ListObjMetas<'a, K>
where
    K: KeyMap,
{
    //
    /// Stable business-object identifiers.
    pub ids: &'a [&'a str],
    /// Compile-time object marker selected for this operation.
    #[doc(hidden)]
    _m: PhantomData<fn() -> K>,
}

impl<'a, K> ListObjMetas<'a, K>
where
    K: KeyMap,
{
    /// Creates a metadata lookup for the supplied business-object identifiers.
    #[must_use]
    pub const fn new(ids: &'a [&'a str]) -> Self {
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
pub struct GenObjUrls<'a, K>
where
    K: KeyMap,
{
    //
    /// Metadata versions whose physical keys will be resolved.
    pub metas: &'a HashMap<String, ObjMeta>,
    /// Read-URL renditions selected for this operation.
    pub spec: ObjUrlSpec,
    /// Compile-time object marker selected for this operation.
    #[doc(hidden)]
    _m: PhantomData<fn() -> K>,
}

impl<'a, K> GenObjUrls<'a, K>
where
    K: KeyMap,
{
    /// Creates a read-URL request for the supplied object metadata.
    #[must_use]
    pub const fn new(
        metas: &'a HashMap<String, ObjMeta>,
        spec: ObjUrlSpec,
    ) -> Self {
        //
        Self {
            metas,
            spec,
            _m: PhantomData,
        }
    }
}

/// Allocates or resumes one generation and its locally signed write capability.
#[derive(Oper)]
#[oper(output = Option<ObjSlot>)]
pub struct GenObjSlot<'a, K>
where
    K: KeyMap,
{
    //
    /// Business-planned object and write requirements.
    pub spec: &'a ObjSlotSpec<'a, K>,
    /// Compile-time object marker selected for this operation.
    #[doc(hidden)]
    _m: PhantomData<fn() -> K>,
}

impl<'a, K> GenObjSlot<'a, K>
where
    K: KeyMap,
{
    /// Creates an allocation request for one business object.
    #[must_use]
    pub const fn new(spec: &'a ObjSlotSpec<'a, K>) -> Self {
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
pub struct GenObjSlots<'a, K>
where
    K: KeyMap,
{
    //
    /// Business-planned objects and their write requirements.
    pub specs: &'a [ObjSlotSpec<'a, K>],
    /// Compile-time object marker selected for this operation.
    #[doc(hidden)]
    _m: PhantomData<fn() -> K>,
}

impl<'a, K> GenObjSlots<'a, K>
where
    K: KeyMap,
{
    /// Creates a bulk reservation for the supplied business objects.
    #[must_use]
    pub const fn new(specs: &'a [ObjSlotSpec<'a, K>]) -> Self {
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
pub struct MarkObjUploaded<'a, K>
where
    K: KeyMap,
{
    //
    /// Exact logical object generation declared uploaded by the client.
    pub key: &'a ObjGen,
    /// Compile-time object marker selected for this operation.
    #[doc(hidden)]
    _m: PhantomData<fn() -> K>,
}

impl<'a, K> MarkObjUploaded<'a, K>
where
    K: KeyMap,
{
    /// Creates an upload declaration for one exact object generation.
    #[must_use]
    pub const fn new(key: &'a ObjGen) -> Self {
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
pub struct ClearObjs<'a, K>
where
    K: KeyMap,
{
    //
    /// Business-object identifiers whose current files are cleared.
    pub ids: &'a [String],
    /// Compile-time object marker selected for this operation.
    #[doc(hidden)]
    _m: PhantomData<fn() -> K>,
}

impl<'a, K> ClearObjs<'a, K>
where
    K: KeyMap,
{
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
pub struct DeleteObjs<'a, K>
where
    K: KeyMap,
{
    //
    /// Permanently retired business-object identifiers.
    pub ids: &'a [String],
    /// Compile-time object marker selected for this operation.
    #[doc(hidden)]
    _m: PhantomData<fn() -> K>,
}

impl<'a, K> DeleteObjs<'a, K>
where
    K: KeyMap,
{
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
