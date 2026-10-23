//! Shared values and dependency access for generated typed RDB operations.

use diesel::result::{DatabaseErrorKind, Error as DieselError};

use poprako_rdb_core::RdbError;

use crate::key::{KeyMap, ObjKey};
use crate::model::meta::ObjMeta;
use crate::rest::{ObjDeptError, ObjDeptRest};

/// Standardized latest object record decoded from a concrete Diesel table.
#[derive(Clone, PartialEq, Eq)]
pub struct ObjRdbRecord {
    /// Stored generation watermark.
    pub ver: i64,

    /// Complete physical key, or none for a detached record.
    pub key: Option<String>,
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
    /// Complete immutable physical object key.
    pub key: &'a str,
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
#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
pub fn decode_record<K>(
    id: &str,
    record: ObjRdbRecord,
) -> ObjDeptRest<Option<ObjMeta>>
where
    K: KeyMap<Img = String>,
{
    //
    let ver =
        u32::try_from(record.ver).map_err(|_| ObjDeptError::Unrecoverable {
            message: "object ver is outside u32".into(),
        })?;

    match (record.key, record.f_is_uploaded, record.hash, record.ext) {
        //
        (None, None, None, None) => Ok(None),

        (Some(key), Some(is_avail), Some(hash), Some(ext)) => {
            //
            validate_key::<K>(id, ver, &ext, &key)?;

            //
            Ok(Some(ObjMeta {
                key: ObjKey {
                    id: id.to_owned(),
                    ver,
                    image: key,
                },
                is_avail,
                hash,
                ext,
            }))
        }

        _ => Err(ObjDeptError::Unrecoverable {
            message: format!("invalid object row: {}", id),
        }),
    }
}

/// Allocates the next ver after validating the current record.
///
/// # Errors
///
/// Returns an unrecoverable error for an invalid record or ver overflow.
#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
pub fn next_ver(id: &str, record: Option<&ObjRdbRecord>) -> ObjDeptRest<u32> {
    //
    let ver = match record {
        //
        Some(record) => {
            //
            match (
                &record.key,
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
                        message: format!("invalid object row: {}", id),
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
#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
pub fn active_key<K>(
    id: &str,
    record: Option<&ObjRdbRecord>,
) -> ObjDeptRest<Option<ObjKey>>
where
    K: KeyMap<Img = String>,
{
    //
    let Some(record) = record else {
        return Ok(None);
    };

    match (
        &record.key,
        &record.f_is_uploaded,
        &record.hash,
        &record.ext,
    ) {
        //
        (Some(key), Some(_), Some(_), Some(ext)) => {
            //
            let ver = u32::try_from(record.ver).map_err(|_| {
                ObjDeptError::Unrecoverable {
                    message: "object ver is outside u32".into(),
                }
            })?;

            validate_key::<K>(id, ver, ext, key)?;

            Ok(Some(ObjKey {
                id: id.to_owned(),
                ver,
                image: key.clone(),
            }))
        }

        (None, None, None, None) => Ok(None),

        _ => Err(ObjDeptError::Unrecoverable {
            message: format!("invalid object row: {}", id),
        }),
    }
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
#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
pub fn rdb_error(source: RdbError) -> ObjDeptError {
    //
    match source {
        //
        RdbError::PoolBuild { source } => ObjDeptError::Retryable {
            message: format!("failed to build RDB pool: {}", source),
        },

        RdbError::PoolGet { message } => ObjDeptError::Retryable {
            message: format!("failed to acquire RDB connection: {}", message),
        },

        RdbError::PoolWaitTimeout => ObjDeptError::Unavailable {
            message: "timed out waiting for an RDB connection".into(),
        },
    }
}

// Validates a stored physical key against its relational metadata.
#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
fn validate_key<K>(id: &str, ver: u32, ext: &str, key: &str) -> ObjDeptRest<()>
where
    K: KeyMap<Img = String>,
{
    //
    let image = key.to_owned();

    let (dom, decoded_ver) =
        K::reverse(&image).map_err(|_| ObjDeptError::Unrecoverable {
            message: format!("invalid object key: {}", id),
        })?;

    let is_consistent = K::id(&dom) == id
        && K::ext(&dom) == ext
        && decoded_ver == ver
        && K::forward(&dom, decoded_ver) == image;

    match () {
        //
        () if is_consistent => Ok(()),

        () => Err(ObjDeptError::Unrecoverable {
            message: format!("inconsistent object key: {}", id),
        }),
    }
}
