//! Error construction and source-level tracing shared by application modules.

// Constructs an expected error and records one warning at its unlogged source.
//
// Supply the variant, `message: String;`, then native tracing fields and a
// static business description. The variant and message are evaluated once;
// additional fields retain tracing's lazy evaluation when logging is disabled.
// `err_variant`, `err_message`, `source_file`, and `source_line` are reserved.
// Use `Err(...)` at return sites and the error directly in lazy error closures.
// Already-logged conversions and test fixtures may construct the enum directly;
// propagation must not emit another event for the same failure.
macro_rules! expected_error {
    ($variant:expr, message: $message:expr; $($fields:tt)*) => {{
        let variant: $crate::result::ExpectedVariant = $variant;

        let message: ::std::string::String = $message;

        tracing::warn!(
            err_variant = ?variant,
            err_message = %message,
            source_file = file!(),
            source_line = line!(),
            $($fields)*
        );

        $crate::result::BaseError::Expected { variant, msg: message }
    }};
}

// Constructs a retryable error and records one warning at its unlogged source.
//
// Supply `message: String;`, then native tracing fields and a static business
// description. The message is evaluated once; additional fields retain tracing's
// lazy evaluation when logging is disabled. Use safe client-visible messages.
// `err_variant`, `err_message`, `source_file`, and `source_line` are reserved.
// The macro returns BaseError without retrying or changing control flow.
// Already-logged conversions and test fixtures may construct the enum directly.
macro_rules! retryable_error {
    (message: $message:expr; $($fields:tt)*) => {{
        let message: ::std::string::String = $message;

        tracing::warn!(
            err_variant = "Retryable",
            err_message = %message,
            source_file = file!(),
            source_line = line!(),
            $($fields)*
        );

        $crate::result::BaseError::Retryable { msg: message }
    }};
}

// Constructs an unrecoverable error and records one error event at its source.
//
// Supply `message: String;`, then native tracing fields and a static business
// description. The message is evaluated once; additional fields retain tracing's
// lazy evaluation when logging is disabled. Messages and fields must be safe to
// log; external conversions must retain redacted original error diagnostics.
// `err_variant`, `err_message`, `source_file`, and `source_line` are reserved.
// The macro returns BaseError; HTTP conversion conceals its internal message.
// Already-logged conversions and test fixtures may construct the enum directly.
macro_rules! unrecoverable_error {
    (message: $message:expr; $($fields:tt)*) => {{
        let message: ::std::string::String = $message;

        tracing::error!(
            err_variant = "Unrecoverable",
            err_message = %message,
            source_file = file!(),
            source_line = line!(),
            $($fields)*
        );

        $crate::result::BaseError::Unrecoverable { msg: message }
    }};
}
