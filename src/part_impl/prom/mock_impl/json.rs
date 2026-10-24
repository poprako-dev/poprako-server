use crate::result::BaseError;

/// Converts a mock prom payload serialization failure after recording the SDK error.
pub fn serialize_payload_error(error_serde: &serde_json::Error) -> BaseError {
    //
    tracing::error!(
        operation = "serialize_prom_payload",
        sdk_error = ?error_serde,
        "JSON SDK serialization error",
    );

    BaseError::Unrecoverable {
        message: [
            "failed to serialize prom payload: ",
            &error_serde.to_string(),
        ]
        .concat(),
    }
}
