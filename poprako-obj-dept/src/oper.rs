use std::collections::HashMap;
use std::marker::PhantomData;

use poprako_orchestra::Oper;

use crate::key::ObjKey;
use crate::model::mark::MarkObjUploadedOutcome as ObjMarkUploadedOutcome;
use crate::model::meta::ObjMeta;
use crate::model::slot::{ObjSlot, ObjSlotSpec};
use crate::model::url::ObjUrls;

/// Result of marking an exact current object generation as uploaded.
pub type MarkObjUploadedOutcome = ObjMarkUploadedOutcome;

/// Reads current object metadata for a collection of business objects.
#[derive(Oper)]
#[oper(output = HashMap<String, ObjMeta>)]
pub struct ListObjMetas<'a, B> {
    //
    /// Stable business-object identifiers.
    pub ids: &'a [String],
    /// Compile-time object marker selected for this operation.
    #[doc(hidden)]
    pub _m: PhantomData<fn() -> B>,
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
    pub _m: PhantomData<fn() -> B>,
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
    pub _m: PhantomData<fn() -> B>,
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
    pub _m: PhantomData<fn() -> B>,
}

/// Optimistically marks one exact current object generation as uploaded.
#[derive(Oper)]
#[oper(output = MarkObjUploadedOutcome)]
pub struct MarkObjUploaded<'a, B> {
    //
    /// Exact logical object generation declared uploaded by the client.
    pub key: &'a ObjKey,
    /// Compile-time object marker selected for this operation.
    #[doc(hidden)]
    pub _m: PhantomData<fn() -> B>,
}

/// Reliably retires current objects inside the caller-owned transaction.
#[derive(Oper)]
#[oper(output = ())]
pub enum RetireObjs<'a, B> {
    //
    /// Defers physical deletion and retains each identifier's version watermark.
    PreserveWatermarks {
        /// Business-object identifiers to detach.
        ids: &'a [String],
        /// Compile-time object marker selected for this operation.
        #[doc(hidden)]
        _m: PhantomData<fn() -> B>,
    },

    /// Defers physical deletion and removes each object row.
    ///
    /// The supplied business identifiers must never be reused within this
    /// object topic; otherwise removing the watermark permits an ABA race.
    RemoveRows {
        /// Business-object identifiers whose rows are removed.
        ids: &'a [String],
        /// Compile-time object marker selected for this operation.
        #[doc(hidden)]
        _m: PhantomData<fn() -> B>,
    },
}
