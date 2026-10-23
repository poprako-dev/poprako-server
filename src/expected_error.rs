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
