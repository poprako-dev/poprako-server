#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

//! Error construction records one event without changing propagation semantics.

mod failures;

use std::cell::Cell;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

use time::OffsetDateTime;
use tracing::field::{Field, Visit};
use tracing::level_filters::LevelFilter;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::{Context, SubscriberExt as _};
use tracing_subscriber::registry::LookupSpan;

use crate::complex::chapter_port::perm as chapter_port_perm_complex;
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::result::{BaseError, BaseRest, ExpectedVariant};
use crate::test_util::http_logging::capture_dispatch;
use crate::test_util::{assert_expected_message, assert_expected_variant};
use crate::value::role::RoleField;

#[derive(Default)]
struct Fields(BTreeMap<String, String>);

impl Visit for Fields {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        self.0.insert(field.name().into(), format!("{value:?}"));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.insert(field.name().into(), value.into());
    }
}

struct RecordedEvent {
    level: Level,
    target: String,
    span: Option<String>,

    file: Option<String>,
    line: Option<u32>,

    fields: Fields,
}

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<RecordedEvent>>>);

impl<S> Layer<S> for Capture
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, context: Context<'_, S>) {
        let metadata = event.metadata();

        let mut fields = Fields::default();

        event.record(&mut fields);

        self.0.lock().unwrap().push(RecordedEvent {
            level: *metadata.level(),
            target: metadata.target().into(),
            span: context.lookup_current().map(|span| span.name().into()),
            file: metadata.file().map(str::to_owned),
            line: metadata.line(),
            fields,
        });
    }
}

fn capture<T>(
    level: LevelFilter,
    operation: impl FnOnce() -> T,
) -> (T, Vec<RecordedEvent>) {
    let capture = Capture::default();

    let subscriber = tracing_subscriber::registry()
        .with(capture.clone())
        .with(level);

    let result = tracing::dispatcher::with_default(
        &capture_dispatch(subscriber),
        operation,
    );

    let events = std::mem::take(&mut *capture.0.lock().unwrap());

    (result, events)
}

fn reject_request() -> BaseRest<()> {
    Err(expected_error! {
        ExpectedVariant::Perm,
        message: "permission required".into();
        "request permission required"
    })
}

fn propagate_rejection() -> BaseRest<()> {
    reject_request()?;

    accept!(())
}

#[test]
fn expected_errors_record_classification_fields_and_call_site() {
    for variant in [
        ExpectedVariant::Args,
        ExpectedVariant::Auth,
        ExpectedVariant::Perm,
    ] {
        let ((error, source_line), events) = capture(LevelFilter::WARN, || {
            let span = tracing::warn_span!("business_operation");

            let _entered = span.enter();

            let source_line = line!() + 2;

            let error = expected_error! {
                variant,
                message: "localized detail".into();
                user_id = %"user-1",
                candidates = ?["candidate-1"],
                count = 7_u64,
                "business rule rejected",
            };

            (error, source_line)
        });

        assert_expected_variant(&error, variant);

        assert!(matches!(error, BaseError::Expected { message, .. }
            if message == "localized detail"));

        assert_eq!(events.len(), 1);

        let event = events.first().unwrap();

        assert_eq!(event.level, Level::WARN);

        assert_eq!(event.target, module_path!());

        assert_eq!(event.span.as_deref(), Some("business_operation"));

        assert_eq!(event.file.as_deref(), Some(file!()));

        assert_eq!(event.line, Some(source_line));

        let fields = &event.fields.0;

        for (name, value) in [
            ("error_variant", format!("{variant:?}")),
            ("error_message", "localized detail".into()),
            ("source_file", file!().into()),
            ("source_line", source_line.to_string()),
            ("user_id", "user-1".into()),
            ("candidates", "[\"candidate-1\"]".into()),
            ("count", "7".into()),
            ("message", "business rule rejected".into()),
        ] {
            assert_eq!(fields.get(name), Some(&value), "{name}");
        }

        assert!(!fields.contains_key("err_msg"));
    }
}

#[test]
fn error_arguments_are_evaluated_once_and_log_fields_are_lazy() {
    for (level, expected_events) in
        [(LevelFilter::WARN, 1), (LevelFilter::OFF, 0)]
    {
        let variant_calls = Cell::new(0);

        let message_calls = Cell::new(0);

        let field_calls = Cell::new(0);

        let (error, events) = capture(level, || {
            expected_error! {
                {
                    variant_calls.set(variant_calls.get() + 1);

                    ExpectedVariant::Args
                },
                message: {
                    message_calls.set(message_calls.get() + 1);

                    "invalid input".into()
                };
                detail = {
                    field_calls.set(field_calls.get() + 1);

                    "diagnostic"
                },
                "input rejected",
            }
        });

        assert_expected_variant(&error, ExpectedVariant::Args);

        assert!(matches!(error, BaseError::Expected { message, .. }
            if message == "invalid input"));

        assert_eq!(variant_calls.get(), 1);

        assert_eq!(message_calls.get(), 1);

        assert_eq!(field_calls.get(), expected_events);

        assert_eq!(events.len(), expected_events);
    }
}

#[test]
fn lazy_error_closures_only_log_failed_operations() {
    let ((success, failure), events) = capture(LevelFilter::WARN, || {
        let success = Some(()).ok_or_else(|| {
            expected_error! {
                ExpectedVariant::Args,
                message: "missing value".into();
                "value required",
            }
        });

        let success = success.map_err(|_| {
            expected_error! {
                ExpectedVariant::Args,
                message: "unexpected failure".into();
                "operation failed",
            }
        });

        let failure = None::<()>.ok_or_else(|| {
            expected_error! {
                ExpectedVariant::Args,
                message: "missing value".into();
                "value required",
            }
        });

        (success, failure)
    });

    assert!(success.is_ok());

    assert_expected_variant(&failure.unwrap_err(), ExpectedVariant::Args);

    assert_eq!(events.len(), 1);
}

#[test]
fn external_error_mapping_retains_original_diagnostics() {
    let (result, events) = capture(LevelFilter::WARN, || {
        "invalid".parse::<u32>().map_err(|source| {
            expected_error! {
                ExpectedVariant::Args,
                message: "invalid number".into();
                parse_error = ?source,
                "number parsing failed",
            }
        })
    });

    assert_expected_variant(&result.unwrap_err(), ExpectedVariant::Args);

    assert_eq!(events.len(), 1);

    let event = events.first().unwrap();

    assert!(
        event
            .fields
            .0
            .get("parse_error")
            .unwrap()
            .contains("InvalidDigit")
    );
}

#[test]
fn propagation_does_not_record_another_event() {
    let (result, events) = capture(LevelFilter::WARN, propagate_rejection);

    assert_expected_variant(&result.unwrap_err(), ExpectedVariant::Perm);

    assert_eq!(events.len(), 1);
}

#[test]
fn permission_message_remapping_preserves_one_source_event() {
    let assignment_info = AssignmentInfo {
        id: "assignment-1".into(),
        chapter_id: "chapter-1".into(),
        user_id: "user-1".into(),
        user: None,
        chapter: None,
        roles: RoleField::RAW_PROVIDER.into(),
        created_at: OffsetDateTime::UNIX_EPOCH,
        updated_at: OffsetDateTime::UNIX_EPOCH,
    };

    let (result, events) = capture(LevelFilter::WARN, || {
        chapter_port_perm_complex::ensure_user_can_import_translation(
            &assignment_info,
        )
    });

    assert_expected_message(
        result.unwrap_err(),
        ExpectedVariant::Perm,
        "error-chapter-port-import-perm-required",
    );

    assert_eq!(events.len(), 1);
}
