//! Shared values and dependency access for generated typed RDB operations.

use diesel::result::{DatabaseErrorKind, Error as DieselError};

use poprako_rdb_core::RdbError;

use crate::key::ObjKey;
use crate::model::meta::ObjMeta;
use crate::rest::{ObjDeptError, ObjDeptRest};

/// Standardized latest object row decoded from a concrete Diesel table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjRdbRow {
    /// Stored generation watermark.
    pub version: i64,
    /// Current upload evidence, or none for a detached row.
    pub f_is_uploaded: Option<bool>,
    /// Current content hash, or none for a detached row.
    pub hash: Option<Vec<u8>>,
    /// Current object suffix, or none for a detached row.
    pub ext: Option<String>,
}

/// Pending latest object values written by `GenObjSlot`.
#[derive(Debug, Clone, Copy)]
pub struct ObjRdbWrite<'a> {
    /// Stable business-object identifier.
    pub id: &'a str,
    /// Newly allocated generation.
    pub version: u32,
    /// Validated content hash.
    pub hash: &'a [u8],
    /// Validated object suffix.
    pub ext: &'a str,
}

/// Converts one concrete typed row into public object metadata.
///
/// # Errors
///
/// Returns an unrecoverable error when the row is inconsistent or out of range.
pub fn decode_row(id: &str, row: ObjRdbRow) -> ObjDeptRest<Option<ObjMeta>> {
    //
    let version = u32::try_from(row.version).map_err(|_| {
        //
        ObjDeptError::Unrecoverable {
            message: "object version is outside u32".into(),
        }
    })?;

    match (row.f_is_uploaded, row.hash, row.ext) {
        //
        (None, None, None) => Ok(None),

        (Some(f_is_uploaded), Some(hash), Some(ext)) => {
            //
            Ok(Some(ObjMeta {
                key: ObjKey {
                    id: id.to_owned(),
                    version,
                },
                f_is_uploaded,
                hash,
                ext,
            }))
        }

        _ => Err(ObjDeptError::Unrecoverable {
            message: format!("invalid object row: {}", id),
        }),
    }
}

/// Allocates the next version after validating the current row.
///
/// # Errors
///
/// Returns an unrecoverable error for an invalid row or version overflow.
pub fn next_version(id: &str, row: Option<&ObjRdbRow>) -> ObjDeptRest<u32> {
    //
    let version = match row {
        //
        Some(row) => {
            //
            decode_row(id, row.clone())?;

            u32::try_from(row.version).map_err(|_| {
                //
                ObjDeptError::Unrecoverable {
                    message: "object version is outside u32".into(),
                }
            })?
        }

        None => 0,
    };

    version
        .checked_add(1)
        .ok_or_else(|| ObjDeptError::Unrecoverable {
            message: "object version overflow".into(),
        })
}

/// Rebuilds the exact active logical key from a validated row.
///
/// # Errors
///
/// Returns an unrecoverable error when the row is inconsistent or out of range.
pub fn active_key(
    id: &str,
    row: Option<&ObjRdbRow>,
) -> ObjDeptRest<Option<ObjKey>> {
    //
    let Some(row) = row else {
        return Ok(None);
    };

    match (&row.f_is_uploaded, &row.hash, &row.ext) {
        //
        (Some(_), Some(_), Some(_)) => {
            //
            let version = u32::try_from(row.version).map_err(|_| {
                //
                ObjDeptError::Unrecoverable {
                    message: "object version is outside u32".into(),
                }
            })?;

            Ok(Some(ObjKey {
                id: id.to_owned(),
                version,
            }))
        }

        (None, None, None) => Ok(None),

        _ => Err(ObjDeptError::Unrecoverable {
            message: format!("invalid object row: {}", id),
        }),
    }
}

/// Maps and traces a Diesel adapter failure.
pub fn diesel_err(source: DieselError) -> ObjDeptError {
    //
    tracing::error!(
        operation = "access_obj_dept_rdb",
        sdk_err = ?source,
        "Diesel SDK error",
    );

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

/// Maps and traces an RDB pool failure.
pub fn rdb_err(source: RdbError) -> ObjDeptError {
    //
    tracing::error!(
        operation = "get_obj_dept_rdb_conn",
        sdk_err = ?source,
        "RDB pool error",
    );

    let message = match source {
        //
        RdbError::PoolBuild { source } => {
            format!("failed to build RDB pool: {}", source)
        }

        RdbError::PoolGet { message } => {
            format!("failed to acquire RDB connection: {}", message)
        }
    };

    ObjDeptError::Retryable { message }
}
