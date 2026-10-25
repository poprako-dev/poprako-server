//! Shared values and dependency access for generated typed RDB operations.

use diesel::result::{DatabaseErrorKind, Error as DieselError};

use poprako_rdb_core::RdbError;

use crate::key::{ObjId as _, ObjKey, ObjKind};
use crate::model::meta::ObjMeta;
use crate::model::record::{ObjKeyRecord, ObjMetaRecord};
use crate::rest::{ObjDeptError, ObjDeptRest};

/// Standardized latest object record decoded from a concrete Diesel table.
#[derive(Clone, PartialEq, Eq)]
pub struct ObjRdbRecord {
    /// Stored generation watermark.
    pub ver: i64,

    /// Complete construction data, or none for a detached record.
    pub domain: Option<serde_json::Value>,
    /// Current upload evidence, or none for a detached record.
    pub f_is_uploaded: Option<bool>,
    /// Current content hash, or none for a detached record.
    pub hash: Option<Vec<u8>>,
    /// Current object suffix, or none for a detached record.
    pub ext: Option<String>,
}

/// Next active object values written by `GenObjSlot`.
#[derive(Clone, Copy)]
pub struct ObjRdbWrite<'a> {
    /// Stable business-object identifier.
    pub id: &'a str,

    /// Newly allocated generation.
    pub ver: u32,
    /// Complete validated construction data.
    pub domain: &'a serde_json::Value,
    /// Validated content hash.
    pub hash: &'a [u8],
    /// Validated object suffix.
    pub ext: &'a str,
}

/// Converts one concrete typed record into public object metadata.
///
/// # Errors
///
/// Returns an unrecoverable error when the record is inconsistent or out of range.
pub fn decode_record<K>(
    id: &str,
    record: ObjRdbRecord,
) -> ObjDeptRest<Option<ObjMeta<K>>>
where
    K: ObjKind,
{
    //
    let _ = K::Id::from_text(id).map_err(|_| ObjDeptError::Unrecoverable {
        message: "invalid persisted object identity".into(),
    })?;

    let ver =
        u32::try_from(record.ver).map_err(|_| ObjDeptError::Unrecoverable {
            message: "object ver is outside u32".into(),
        })?;

    match (record.domain, record.f_is_uploaded, record.hash, record.ext) {
        //
        (None, None, None, None) => Ok(None),

        (Some(key), Some(is_avail), Some(hash), Some(ext)) => {
            //
            let meta = ObjMetaRecord {
                key: ObjKeyRecord {
                    id: id.into(),
                    ver,
                    domain: key,
                },
                is_avail,
                hash,
                ext,
            }
            .decode::<K>()?;

            Ok(Some(meta))
        }

        _ => Err(ObjDeptError::Unrecoverable {
            message: ["invalid object row: ", id].concat(),
        }),
    }
}

/// Allocates the next ver after validating the current record.
///
/// # Errors
///
/// Returns an unrecoverable error for an invalid record or ver overflow.
pub fn next_ver(id: &str, record: Option<&ObjRdbRecord>) -> ObjDeptRest<u32> {
    //
    let ver = match record {
        //
        Some(record) => {
            //
            match (
                &record.domain,
                &record.f_is_uploaded,
                &record.hash,
                &record.ext,
            ) {
                //
                (None, None, None, None)
                | (Some(_), Some(_), Some(_), Some(_)) => {}

                _ => {
                    //
                    return Err(ObjDeptError::Unrecoverable {
                        message: ["invalid object row: ", id].concat(),
                    });
                }
            }

            u32::try_from(record.ver).map_err(|_| {
                ObjDeptError::Unrecoverable {
                    message: "object ver is outside u32".into(),
                }
            })?
        }

        None => 0,
    };

    ver.checked_add(1)
        .ok_or_else(|| ObjDeptError::Unrecoverable {
            message: "object ver overflow".into(),
        })
}

/// Rebuilds the exact active logical key from a validated record.
///
/// # Errors
///
/// Returns an unrecoverable error when the record is inconsistent or out of range.
pub fn active_key<K>(
    id: &str,
    record: Option<&ObjRdbRecord>,
) -> ObjDeptRest<Option<ObjKey<K>>>
where
    K: ObjKind,
{
    record
        .cloned()
        .map(|record| decode_record::<K>(id, record))
        .transpose()
        .map(|meta| meta.flatten().map(|meta| meta.key))
}

/// Maps and traces a Diesel adapter failure.
pub fn diesel_error(source: DieselError) -> ObjDeptError {
    //
    match &source {
        //
        DieselError::DatabaseError(
            DatabaseErrorKind::SerializationFailure,
            _,
        ) => {
            //
            tracing::warn!(
                operation = "access_obj_dept_rdb",
                sdk_error = ?source,
                "retryable Diesel SDK error",
            );
        }

        _ => {
            //
            tracing::error!(
                operation = "access_obj_dept_rdb",
                sdk_error = ?source,
                "Diesel SDK error",
            );
        }
    }

    match source {
        //
        DieselError::DatabaseError(
            DatabaseErrorKind::SerializationFailure,
            info,
        ) => ObjDeptError::Retryable {
            message: info.message().to_owned(),
        },

        source => ObjDeptError::Unrecoverable {
            message: source.to_string(),
        },
    }
}

/// Maps an RDB pool failure already traced by its production adapter.
#[must_use]
pub fn rdb_error(source: RdbError) -> ObjDeptError {
    //
    match source {
        //
        RdbError::PoolBuild { source } => ObjDeptError::Retryable {
            message: ["failed to build RDB pool: ", &source.to_string()]
                .concat(),
        },

        RdbError::PoolGet { message } => ObjDeptError::Retryable {
            message: ["failed to acquire RDB connection: ", &message].concat(),
        },

        RdbError::PoolWaitTimeout => ObjDeptError::Unavailable {
            message: "timed out waiting for an RDB connection".into(),
        },
    }
}
