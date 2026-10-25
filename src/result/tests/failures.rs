//! Internal failures retain diagnostics while propagation remains silent.

use std::cell::Cell;

use diesel::result::{DatabaseErrorKind, Error as DieselError};
use poprako_obj_dept::rest::ObjDeptError;
use tracing::Level;
use tracing::level_filters::LevelFilter;

use super::capture;
use crate::complex::unit::sequence as unit_sequence_complex;
use crate::model::read::proj::issue::IssueInfo;
use crate::part_impl::auth::jwt_impl::JwtAuth;
use crate::part_impl::repo::rdb_impl::entity::issue::IssueRecord;
use crate::result::BaseError;
use crate::shared::result::diesel;

fn issue_record(index: i32) -> IssueRecord {
    IssueRecord {
        f_id: "issue--00000000001".into(),
        f_page_artwork_id: "page_artwork--00000000001".into(),
        f_index: index,
        f_variant: "review".into(),
        f_layer_name: Some("private-layer-sentinel".into()),
        f_x_coord: None,
        f_y_coord: None,
        f_width: None,
        f_height: None,
        f_note: "private-review-sentinel".into(),
    }
}

#[test]
fn failure_macros_record_level_fields_and_call_site() {
    for (retryable, variant, level) in [
        (false, "Unrecoverable", Level::ERROR),
        (true, "Retryable", Level::WARN),
    ] {
        let ((error, source_line), events) = capture(LevelFilter::WARN, || {
            let span = tracing::error_span!("business_operation");

            let _entered = span.enter();

            if retryable {
                let source_line = line!() + 2;

                let error = retryable_error! {
                    message: "safe diagnostic".into();
                    resource_id = %"resource-1",
                    reason = ?Some("conflict"),
                    "transaction conflicted",
                };

                return (error, source_line);
            }

            let source_line = line!() + 2;

            let error = unrecoverable_error! {
                message: "safe diagnostic".into();
                resource_id = %"resource-1",
                reason = ?Some("corruption"),
                "stored state is invalid",
            };

            (error, source_line)
        });

        assert!(matches!(
            (&error, retryable),
            (BaseError::Retryable { .. }, true)
                | (BaseError::Unrecoverable { .. }, false)
        ));

        assert!(matches!(error,
            BaseError::Retryable { message }
                | BaseError::Unrecoverable { message }
                if message == "safe diagnostic"));

        assert_eq!(events.len(), 1);

        let event = events.first().unwrap();

        assert_eq!(event.level, level);

        assert_eq!(event.target, module_path!());

        assert_eq!(event.span.as_deref(), Some("business_operation"));

        assert_eq!(event.file.as_deref(), Some(file!()));

        assert_eq!(event.line, Some(source_line));

        for (name, value) in [
            ("error_variant", variant.to_owned()),
            ("error_message", "safe diagnostic".into()),
            ("resource_id", "resource-1".into()),
            ("source_file", file!().into()),
            ("source_line", source_line.to_string()),
        ] {
            assert_eq!(event.fields.0.get(name), Some(&value), "{name}");
        }

        assert!(event.fields.0.contains_key("reason"));

        assert!(!event.fields.0.contains_key("err_msg"));
    }
}

#[test]
fn failure_messages_are_evaluated_once_when_logging_is_filtered() {
    for retryable in [false, true] {
        for (level, expected_events) in
            [(LevelFilter::WARN, 1), (LevelFilter::OFF, 0)]
        {
            let message_calls = Cell::new(0);

            let field_calls = Cell::new(0);

            let (error, events) = capture(level, || {
                if retryable {
                    return retryable_error! {
                        message: {
                            message_calls.set(message_calls.get() + 1);

                            "diagnostic".into()
                        };
                        count = {
                            field_calls.set(field_calls.get() + 1);

                            1_u64
                        },
                        "transaction conflicted"
                    };
                }

                unrecoverable_error! {
                    message: {
                        message_calls.set(message_calls.get() + 1);

                        "diagnostic".into()
                    };
                    count = {
                        field_calls.set(field_calls.get() + 1);

                        1_u64
                    },
                    "stored state is invalid"
                }
            });

            assert!(matches!(error,
                BaseError::Retryable { message }
                    | BaseError::Unrecoverable { message }
                    if message == "diagnostic"));

            assert_eq!(message_calls.get(), 1);

            assert_eq!(field_calls.get(), expected_events);

            assert_eq!(events.len(), expected_events);
        }
    }
}

#[test]
fn diesel_conflicts_and_internal_failures_retain_source_diagnostics() {
    for (source, expected_level, field, diagnostic) in [
        (
            DieselError::DatabaseError(
                DatabaseErrorKind::SerializationFailure,
                Box::new("serialization diagnostic".to_string()),
            ),
            Level::WARN,
            "database_message",
            "serialization diagnostic",
        ),
        (
            DieselError::RollbackTransaction,
            Level::ERROR,
            "sdk_error",
            "RollbackTransaction",
        ),
    ] {
        let (error, events) = capture(LevelFilter::WARN, || diesel(source));

        assert!(matches!(
            (error, expected_level),
            (BaseError::Retryable { .. }, Level::WARN)
                | (BaseError::Unrecoverable { .. }, Level::ERROR)
        ));

        assert_eq!(events.len(), 1);

        let event = events.first().unwrap();

        assert_eq!(event.level, expected_level);

        assert_eq!(event.fields.0.get(field).unwrap(), diagnostic);

        assert!(event.fields.0.contains_key("error_message"));
    }
}

#[test]
fn invalid_jwt_lifetimes_log_without_the_signing_secret() {
    for expiration_hours in [0, -1, i64::MAX] {
        let (result, events) = capture(LevelFilter::ERROR, || {
            JwtAuth::new("private-signing-secret-sentinel", expiration_hours)
        });

        assert!(matches!(result, Err(BaseError::Unrecoverable { .. })));

        assert_eq!(events.len(), 1);

        let event = events.first().unwrap();

        assert_eq!(event.level, Level::ERROR);

        assert_eq!(
            event.fields.0.get("expiration_hours"),
            Some(&expiration_hours.to_string()),
        );

        assert!(event.fields.0.values().all(|value| {
            !value.contains("private-signing-secret-sentinel")
        }));
    }
}

#[test]
fn incomplete_stored_rectangles_log_ids_without_private_content() {
    let mut row = issue_record(0);

    row.f_x_coord = Some(0.1);

    let (result, events) =
        capture(LevelFilter::ERROR, || IssueInfo::try_from(row));

    assert!(matches!(result, Err(BaseError::Unrecoverable { .. })));

    assert_eq!(events.len(), 1);

    let event = events.first().unwrap();

    assert_eq!(
        event.fields.0.get("issue_id").unwrap(),
        "issue--00000000001"
    );

    assert_eq!(
        event.fields.0.get("page_artwork_id").unwrap(),
        "page_artwork--00000000001"
    );

    assert!(event.fields.0.values().all(|value| {
        !value.contains("private-layer-sentinel")
            && !value.contains("private-review-sentinel")
    }));
}

#[test]
fn corrupt_unit_chains_record_one_internal_failure() {
    let mut units = [("unit--00000000001", None), ("unit--00000000001", None)];

    let (result, events) = capture(LevelFilter::ERROR, || {
        unit_sequence_complex::order_units(
            &mut units,
            |unit| unit.0,
            |unit| unit.1,
        )
    });

    assert!(matches!(result, Err(BaseError::Unrecoverable { .. })));

    assert_eq!(events.len(), 1);

    assert_eq!(events.first().unwrap().level, Level::ERROR);
}

#[test]
fn invalid_database_counts_keep_the_field_and_value() {
    let row = issue_record(-1);

    let (result, events) =
        capture(LevelFilter::ERROR, || IssueInfo::try_from(row));

    assert!(matches!(result, Err(BaseError::Unrecoverable { .. })));

    assert_eq!(events.len(), 1);

    let event = events.first().unwrap();

    assert_eq!(event.fields.0.get("field").unwrap(), "t_issue.f_index");

    assert_eq!(event.fields.0.get("value").unwrap(), "-1");
}

#[test]
fn already_logged_retryable_errors_are_converted_without_another_event() {
    let (error, events) = capture(LevelFilter::WARN, || {
        let source = ObjDeptError::retryable("retry object operation".into());

        BaseError::from(source)
    });

    assert!(matches!(error, BaseError::Retryable { message }
        if message == "retry object operation"));

    assert_eq!(events.len(), 1);
}
