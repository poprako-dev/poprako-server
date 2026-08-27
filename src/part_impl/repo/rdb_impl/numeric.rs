use crate::result::{BaseError, BaseRest};

/// Converts a database INTEGER into a non-negative business value.
pub fn usize_from_i32(value: i32, field: &str) -> BaseRest<usize> {
    //
    usize::try_from(value).map_err(|_| {
        //
        tracing::error!(
            field,
            value,
            "unrecoverable error: negative database value"
        );

        BaseError::Unrecoverable {
            message: format!("database field {} must be non-negative", field),
        }
    })
}

/// Converts a business value into a database INTEGER.
pub fn i32_from_usize(value: usize, field: &str) -> BaseRest<i32> {
    //
    i32::try_from(value).map_err(|_| {
        //
        tracing::error!(
            field,
            value,
            "unrecoverable error: database integer overflow"
        );

        BaseError::Unrecoverable {
            message: format!("database field {} exceeds INTEGER range", field),
        }
    })
}

/// Converts a database BIGINT into a non-negative role-mask value.
pub fn u32_from_i64(value: i64, field: &str) -> BaseRest<u32> {
    //
    u32::try_from(value).map_err(|_| {
        //
        tracing::error!(
            field,
            value,
            "unrecoverable error: invalid unsigned database value"
        );

        BaseError::Unrecoverable {
            message: format!("database field {} must fit u32", field),
        }
    })
}
