use std::collections::HashMap;
use std::marker::PhantomData;

use poprako_orchestra::Oper;

use crate::key::{ObjGen, ObjKind};
use crate::model::alloc::ObjAlloc;
use crate::model::meta::ObjMeta;
use crate::model::slot::ObjSlotSpec;
use crate::model::url::{ObjUrlSpec, ObjUrls};

/// Reads current object metadata for a collection of business objects.
#[derive(Oper)]
#[oper(output = HashMap<K::Id, ObjMeta<K>>)]
pub struct ListObjMetas<'a, K>
where
    K: ObjKind,
{
    /// Stable business-object identifiers.
    pub ids: &'a [K::Id],
    /// Compile-time object marker selected for this operation.
    #[doc(hidden)]
    _m: PhantomData<fn() -> K>,
}

impl<'a, K> ListObjMetas<'a, K>
where
    K: ObjKind,
{
    /// Creates a metadata lookup for the supplied business-object identifiers.
    #[must_use]
    pub const fn new(ids: &'a [K::Id]) -> Self {
        Self {
            ids,
            _m: PhantomData,
        }
    }
}

/// Generates read URLs for the supplied metadata versions.
///
/// Metadata retains its object kind through the read interface.
///
/// ```compile_fail,E0308
/// use std::collections::HashMap;
/// use poprako_obj_dept::key::ObjKind;
/// use poprako_obj_dept::model::meta::ObjMeta;
/// use poprako_obj_dept::model::url::ObjUrlSpec;
/// use poprako_obj_dept::oper::GenObjUrls;
/// fn urls<K: ObjKind, L: ObjKind>(metas: &HashMap<K::Id, ObjMeta<K>>) {
///     let _ = GenObjUrls::<L>::new(metas, ObjUrlSpec::default().with_origin());
/// }
/// ```
///
/// ```
/// use std::collections::HashMap;
/// use poprako_obj_dept::key::ObjKind;
/// use poprako_obj_dept::model::meta::ObjMeta;
/// use poprako_obj_dept::model::url::ObjUrlSpec;
/// use poprako_obj_dept::oper::GenObjUrls;
/// fn urls<K: ObjKind>(metas: &HashMap<K::Id, ObjMeta<K>>) {
///     let _ = GenObjUrls::<K>::new(metas, ObjUrlSpec::default().with_origin());
/// }
/// ```
#[derive(Oper)]
#[oper(output = HashMap<K::Id, ObjUrls>)]
pub struct GenObjUrls<'a, K>
where
    K: ObjKind,
{
    /// Metadata versions whose physical keys will be resolved.
    pub metas: &'a HashMap<K::Id, ObjMeta<K>>,
    /// Read-URL renditions selected for this operation.
    pub spec: ObjUrlSpec,
    /// Compile-time object marker selected for this operation.
    #[doc(hidden)]
    _m: PhantomData<fn() -> K>,
}

impl<'a, K> GenObjUrls<'a, K>
where
    K: ObjKind,
{
    /// Creates a read-URL request for the supplied object metadata.
    #[must_use]
    pub const fn new(
        metas: &'a HashMap<K::Id, ObjMeta<K>>,
        spec: ObjUrlSpec,
    ) -> Self {
        Self {
            metas,
            spec,
            _m: PhantomData,
        }
    }
}

/// Selects one generation, with a signed write capability when upload is needed.
#[derive(Oper)]
#[oper(output = ObjAlloc<K>)]
pub struct GenObjSlot<'a, K>
where
    K: ObjKind,
{
    /// Business-planned object and write requirements.
    pub spec: &'a ObjSlotSpec<'a, K>,
    /// Compile-time object marker selected for this operation.
    #[doc(hidden)]
    _m: PhantomData<fn() -> K>,
}

impl<'a, K> GenObjSlot<'a, K>
where
    K: ObjKind,
{
    /// Creates an allocation request for one business object.
    #[must_use]
    pub const fn new(spec: &'a ObjSlotSpec<'a, K>) -> Self {
        Self {
            spec,
            _m: PhantomData,
        }
    }
}

/// Selects a generation for every input, with capabilities where upload is needed.
#[derive(Oper)]
#[oper(output = HashMap<K::Id, ObjAlloc<K>>)]
pub struct GenObjSlots<'a, K>
where
    K: ObjKind,
{
    /// Business-planned objects and their write requirements.
    pub specs: &'a [ObjSlotSpec<'a, K>],
    /// Compile-time object marker selected for this operation.
    #[doc(hidden)]
    _m: PhantomData<fn() -> K>,
}

impl<'a, K> GenObjSlots<'a, K>
where
    K: ObjKind,
{
    /// Creates a bulk reservation for the supplied business objects.
    #[must_use]
    pub const fn new(specs: &'a [ObjSlotSpec<'a, K>]) -> Self {
        Self {
            specs,
            _m: PhantomData,
        }
    }
}

/// Optimistically marks one exact current object generation as uploaded.
///
/// ```compile_fail,E0308
/// use poprako_obj_dept::key::{ObjGen, ObjKind};
/// use poprako_obj_dept::oper::MarkObjUploaded;
/// fn mark<K: ObjKind, L: ObjKind>(generation: &ObjGen<K>) {
///     let _ = MarkObjUploaded::<L>::new(generation);
/// }
/// ```
///
/// ```
/// use poprako_obj_dept::key::{ObjGen, ObjKind};
/// use poprako_obj_dept::oper::MarkObjUploaded;
/// fn mark<K: ObjKind>(generation: &ObjGen<K>) {
///     let _ = MarkObjUploaded::<K>::new(generation);
/// }
/// ```
#[derive(Oper)]
#[oper(output = bool)]
pub struct MarkObjUploaded<'a, K>
where
    K: ObjKind,
{
    /// Exact logical object generation declared uploaded by the client.
    pub key: &'a ObjGen<K>,
    /// Compile-time object marker selected for this operation.
    #[doc(hidden)]
    _m: PhantomData<fn() -> K>,
}

impl<'a, K> MarkObjUploaded<'a, K>
where
    K: ObjKind,
{
    /// Creates an upload declaration for one exact object generation.
    #[must_use]
    pub const fn new(key: &'a ObjGen<K>) -> Self {
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
    K: ObjKind,
{
    /// Business-object identifiers whose current files are cleared.
    pub ids: &'a [K::Id],
    /// Compile-time object marker selected for this operation.
    #[doc(hidden)]
    _m: PhantomData<fn() -> K>,
}

impl<'a, K> ClearObjs<'a, K>
where
    K: ObjKind,
{
    /// Creates a request to clear current files for active business entities.
    #[must_use]
    pub const fn new(ids: &'a [K::Id]) -> Self {
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
    K: ObjKind,
{
    /// Permanently retired business-object identifiers.
    pub ids: &'a [K::Id],
    /// Compile-time object marker selected for this operation.
    #[doc(hidden)]
    _m: PhantomData<fn() -> K>,
}

impl<'a, K> DeleteObjs<'a, K>
where
    K: ObjKind,
{
    /// Creates a request to delete objects for ended business entities.
    #[must_use]
    pub const fn new(ids: &'a [K::Id]) -> Self {
        Self {
            ids,
            _m: PhantomData,
        }
    }
}
