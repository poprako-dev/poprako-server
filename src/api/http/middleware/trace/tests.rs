//! HTTP response classification must not duplicate source error events.

use std::time::Duration;

use axum::http::StatusCode;
use tower_http::classify::ServerErrorsFailureClass;
use tracing::Span;

use super::record_request_failure;
use crate::test_util::http_logging::{LogBuffer, capture_dispatch};

fn capture_failure(failure: ServerErrorsFailureClass) -> String {
    let buffer = LogBuffer::default();

    let writer = buffer.clone();

    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_writer(move || writer.clone())
        .finish();

    tracing::dispatcher::with_default(&capture_dispatch(subscriber), || {
        record_request_failure(failure, Duration::ZERO, &Span::none());
    });

    buffer.contents()
}

#[test]
fn server_error_statuses_do_not_emit_another_error_event() {
    for status in [
        StatusCode::INTERNAL_SERVER_ERROR,
        StatusCode::SERVICE_UNAVAILABLE,
    ] {
        let logs =
            capture_failure(ServerErrorsFailureClass::StatusCode(status));

        assert!(logs.is_empty(), "{logs}");
    }
}

#[test]
fn transport_failures_still_record_diagnostics() {
    let logs = capture_failure(ServerErrorsFailureClass::Error(
        "response body transport failure".into(),
    ));

    assert!(logs.contains("response body transport failure"), "{logs}");

    assert_eq!(logs.matches("ERROR").count(), 1, "{logs}");
}
