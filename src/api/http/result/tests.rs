// http_body_serializes_success_envelope(HttpBody)(positive): emits code zero and data.

use super::*;

use serde_json::json;

#[test]
fn http_body_serializes_success_envelope() {
    //
    let http_body =
        HttpBody::new(StatusCode::CREATED, json!({ "id": "comic_1" }));

    let serialized =
        serde_json::to_value(http_body).expect("http body serializes");

    assert_eq!(
        serialized,
        json!({
            "code": 0,
            "data": {
                "id": "comic_1",
            },
        }),
    );
}

#[test]
fn retryable_error_maps_to_conflict() {
    let http_error = HttpError::from(BaseError::Retryable {
        message: "retry request".to_string(),
    });

    assert_eq!(http_error.status, StatusCode::CONFLICT);
    assert_eq!(http_error.code.get(), 8);
    assert_eq!(http_error.message.as_deref(), Some("retry request"));
}
