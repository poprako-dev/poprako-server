use crate::result::BaseRest;

/// Converts a database INTEGER into a non-negative business value.
pub fn usize_from_i32(value: i32, field: &str) -> BaseRest<usize> {
    //
    usize::try_from(value).map_err(|_| {
        //
        unrecoverable_error! {
            message: [
                "database field ",
                field,
                " must be non-negative",
            ]
            .concat();
            field,
            value,
            "negative database value",
        }
    })
}

/// Converts a business value into a database INTEGER.
pub fn i32_from_usize(value: usize, field: &str) -> BaseRest<i32> {
    //
    i32::try_from(value).map_err(|_| {
        //
        unrecoverable_error! {
            message: [
                "database field ",
                field,
                " exceeds INTEGER range",
            ]
            .concat();
            field,
            value,
            "database integer overflow",
        }
    })
}

/// Converts a database BIGINT into a non-negative business value.
pub fn usize_from_i64(value: i64, field: &str) -> BaseRest<usize> {
    //
    usize::try_from(value).map_err(|_| {
        //
        unrecoverable_error! {
            message: [
                "database field ",
                field,
                " must be non-negative",
            ]
            .concat();
            field,
            value,
            "negative database value",
        }
    })
}

/// Converts a database BIGINT into a non-negative role-mask value.
pub fn u32_from_i64(value: i64, field: &str) -> BaseRest<u32> {
    //
    u32::try_from(value).map_err(|_| {
        //
        unrecoverable_error! {
            message: [
                "database field ",
                field,
                " must fit u32",
            ]
            .concat();
            field,
            value,
            "invalid unsigned database value",
        }
    })
}
