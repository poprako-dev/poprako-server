//! Rejection output must not format private fields or enable unrelated noise.

use std::fmt::{self, Debug};

use diesel_async::pooled_connection::deadpool::BuildError;
use poprako_obj_dept::rdb_impl::rdb_err;
use poprako_obj_dept::rest::ObjDeptError;
use poprako_rdb_core::RdbError;

use super::{RequestLogFormat, request_log_filter};
use crate::test_util::http_logging::LogBuffer;

struct Unformattable;

impl Debug for Unformattable {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        panic!("private field must never be formatted")
    }
}

fn capture(directives: Option<&str>, operation: impl FnOnce()) -> String {
    let buffer = LogBuffer::default();

    let writer = buffer.clone();

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(request_log_filter(directives))
        .with_ansi(false)
        .event_format(RequestLogFormat::new(()))
        .with_writer(move || writer.clone())
        .finish();

    tracing::subscriber::with_default(subscriber, operation);

    buffer.contents()
}

#[test]
fn rejection_fields_exclude_body_message_and_unknown_debug_values() {
    for directives in [None, Some("info"), Some("axum=trace"), Some("trace")] {
        let logs = capture(directives, || {
            let span =
                tracing::info_span!("request", request_id = "safe-request-id");

            let _entered = span.enter();

            tracing::trace!(
                target: "axum::rejection",
                status = 422_u16,
                rejection_type = "axum::extract::rejection::JsonDataError",
                body = ?Unformattable,
                unexpected = ?Unformattable,
                "private-message-sentinel",
            );
        });

        assert!(logs.contains("status=422"), "{logs}");

        assert!(logs.contains("JsonDataError"), "{logs}");

        assert!(logs.contains("error_origin=\"axum\""), "{logs}");

        assert!(logs.contains("safe-request-id"), "{logs}");

        assert!(!logs.contains("private-message-sentinel"), "{logs}");

        assert!(!logs.contains("body="), "{logs}");

        assert!(!logs.contains("unexpected="), "{logs}");

        assert_eq!(logs.lines().count(), 1, "{logs}");
    }
}

#[test]
fn default_filter_keeps_rejections_and_errors_without_axum_trace_noise() {
    let logs = capture(Some("info"), || {
        tracing::trace!(target: "axum::serve", "connection lifecycle noise");

        tracing::debug!(target: "axum::serve", "connection debug noise");

        tracing::error!(target: "axum::serve", operation = "accept", "listener failure");

        tracing::info!(
            resource_id = "safe-resource",
            "ordinary application event"
        );
    });

    assert!(!logs.contains("noise"), "{logs}");

    assert!(logs.contains("listener failure"), "{logs}");

    assert!(logs.contains("operation=\"accept\""), "{logs}");

    assert!(logs.contains("ordinary application event"), "{logs}");

    assert!(logs.contains("resource_id=\"safe-resource\""), "{logs}");
}

#[test]
fn explicit_rejection_filter_override_disables_native_events() {
    let logs = capture(Some("info,axum::rejection=off"), || {
        tracing::trace!(target: "axum::rejection", status = 422_u16, rejection_type = "JsonDataError", body = ?Unformattable);
    });

    assert!(logs.is_empty(), "{logs}");
}

#[test]
fn previously_logged_pool_errors_keep_classification_without_conversion_logs() {
    let logs = capture(Some("trace"), || {
        let source = RdbError::PoolGet {
            message: "connection creation failed".into(),
        };

        let error = rdb_err(source);

        assert_eq!(error, ObjDeptError::Retryable {
            message: "failed to acquire RDB connection: connection creation failed".into(),
        });

        let error = rdb_err(RdbError::PoolBuild {
            source: BuildError::NoRuntimeSpecified,
        });

        assert_eq!(
            error,
            ObjDeptError::Retryable {
                message: format!(
                    "failed to build RDB pool: {}",
                    BuildError::NoRuntimeSpecified
                ),
            }
        );

        let error = rdb_err(RdbError::PoolWaitTimeout);

        assert!(matches!(error, ObjDeptError::Unavailable { .. }));
    });

    assert!(logs.is_empty(), "{logs}");
}
