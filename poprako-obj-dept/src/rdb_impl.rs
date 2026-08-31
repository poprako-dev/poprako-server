//! Shared values and dependency access for generated typed RDB operations.

use diesel::result::{DatabaseErrorKind, Error as DieselError};

use poprako_rdb_core::RdbError;

use crate::key::{KeyMap, ObjKey};
use crate::model::meta::ObjMeta;
use crate::rest::{ObjDeptError, ObjDeptRest};

/// Standardized latest object row decoded from a concrete Diesel table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjRdbRow {
    //
    /// Stored generation watermark.
    pub ver: i64,
    /// Complete physical key, or none for a detached row.
    pub key: Option<String>,
    /// Current upload evidence, or none for a detached row.
    pub f_is_uploaded: Option<bool>,
    /// Current content hash, or none for a detached row.
    pub hash: Option<Vec<u8>>,
    /// Current object suffix, or none for a detached row.
    pub ext: Option<String>,
}

/// Next active object values written by `GenObjSlot`.
#[derive(Debug, Clone, Copy)]
pub struct ObjRdbWrite<'a> {
    //
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

/// Converts one concrete typed row into public object metadata.
///
/// # Errors
///
/// Returns an unrecoverable error when the row is inconsistent or out of range.
pub fn decode_row<K>(id: &str, row: ObjRdbRow) -> ObjDeptRest<Option<ObjMeta>>
where
    K: KeyMap<Img = String>,
{
    //
    let ver = u32::try_from(row.ver).map_err(|_| {
        //
        ObjDeptError::Unrecoverable {
            message: "object ver is outside u32".into(),
        }
    })?;

    match (row.key, row.f_is_uploaded, row.hash, row.ext) {
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

/// Allocates the next ver after validating the current row.
///
/// # Errors
///
/// Returns an unrecoverable error for an invalid row or ver overflow.
pub fn next_ver(id: &str, row: Option<&ObjRdbRow>) -> ObjDeptRest<u32> {
    //
    let ver = match row {
        //
        Some(row) => {
            //
            match (&row.key, &row.f_is_uploaded, &row.hash, &row.ext) {
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

            u32::try_from(row.ver).map_err(|_| {
                //
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

/// Rebuilds the exact active logical key from a validated row.
///
/// # Errors
///
/// Returns an unrecoverable error when the row is inconsistent or out of range.
pub fn active_key<K>(
    id: &str,
    row: Option<&ObjRdbRow>,
) -> ObjDeptRest<Option<ObjKey>>
where
    K: KeyMap<Img = String>,
{
    //
    let Some(row) = row else {
        return Ok(None);
    };

    match (&row.key, &row.f_is_uploaded, &row.hash, &row.ext) {
        //
        (Some(key), Some(_), Some(_), Some(ext)) => {
            //
            let ver = u32::try_from(row.ver).map_err(|_| {
                //
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

// Validates a stored physical key against its relational metadata.
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
