use crate::result::BaseError;

/// Converts a mock prom payload serialization failure after recording the SDK error.
#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
pub fn serialize_payload_err(err_serde: &serde_json::Error) -> BaseError {
    //
    tracing::error!(
        operation = "serialize_prom_payload",
        sdk_err = ?err_serde,
        "JSON SDK serialization error",
    );

    BaseError::Unrecoverable {
        msg: format!("failed to serialize prom payload: {}", err_serde),
    }
}
