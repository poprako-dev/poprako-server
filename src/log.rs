//! Production logging with structured, redacted Axum rejection diagnostics.

#[cfg(test)]
mod tests;

use std::env::var;
use std::fmt::{Debug, Result as FmtResult};

use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::{Format, Full, Writer};
use tracing_subscriber::fmt::time::{FormatTime, SystemTime};
use tracing_subscriber::fmt::{
    FmtContext, FormatEvent, FormatFields, FormattedFields,
};
use tracing_subscriber::registry::LookupSpan;

// Only this target contains the native extractor rejection events.
const REJECTION_TARGET: &str = "axum::rejection";

/// Initialise production logging, including native Axum request rejections.
///
/// `RUST_LOG` overrides the default INFO and targeted rejection TRACE directives.
/// Rejection diagnostics are always recorded, with client values redacted.
pub fn init_log() {
    //
    let directives = var("RUST_LOG").ok();

    tracing_subscriber::fmt()
        .with_env_filter(request_log_filter(directives.as_deref()))
        .with_ansi(cfg!(debug_assertions))
        .event_format(RequestLogFormat::new(SystemTime))
        .init();
}

/// Keeps native rejections enabled under ordinary INFO production directives.
pub fn request_log_filter(directives: Option<&str>) -> EnvFilter {
    //
    EnvFilter::builder().parse_lossy(format!(
        "info,axum::rejection=trace,{}",
        directives.unwrap_or_default(),
    ))
}

/// Formats native rejections using safe fields and delegates other events.
pub struct RequestLogFormat<T> {
    /// Standard output formatting for events outside the rejection target.
    default_format: Format<Full, T>,
    /// Timestamp formatter shared with standard events.
    timer: T,
}

impl<T> RequestLogFormat<T>
where
    T: FormatTime + Clone,
{
    /// Uses the same timer for ordinary and rejection events.
    pub fn new(timer: T) -> Self {
        Self {
            default_format: tracing_subscriber::fmt::format()
                .with_timer(timer.clone()),
            timer,
        }
    }
}

impl<S, N, T> FormatEvent<S, N> for RequestLogFormat<T>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
    T: FormatTime,
{
    // Formats native rejection events with redacted diagnostics.
    fn format_event(
        &self,
        context: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> FmtResult {
        //
        if event.metadata().target() != REJECTION_TARGET {
            return self.default_format.format_event(context, writer, event);
        }

        self.timer.format_time(&mut writer)?;

        // Axum emits native rejections at TRACE; render them as warnings so
        // level-filtered sinks retain client errors without extra directives.
        write!(writer, " WARN ")?;

        if let Some(scope) = context.event_scope() {
            //
            for span in scope.from_root() {
                //
                write!(writer, "{}", span.name())?;

                let extensions = span.extensions();

                if let Some(fields) = extensions.get::<FormattedFields<N>>()
                    && !fields.is_empty()
                {
                    writer.write_char('{')?;

                    writer.write_str(fields.as_str())?;

                    writer.write_char('}')?;
                }

                drop(extensions);

                write!(writer, ":")?;
            }
        }

        write!(
            writer,
            " axum::rejection: HTTP request extraction rejected \
                error_origin=\"axum\""
        )?;

        let mut fields = RejectionFields {
            status: None,
            error_variant: None,
            body: None,
        };

        event.record(&mut fields);

        let RejectionFields {
            status,
            error_variant,
            body,
        } = fields;

        if let Some(status) = status {
            write!(writer, " status={}", &status)?;
        }

        if let Some(error_variant) = error_variant.as_ref() {
            write!(writer, " error_variant={:?}", error_variant.as_str())?;
        }

        if let Some(body) = body.as_deref() {
            //
            let message = rejection_message(error_variant.as_deref(), body);

            write!(writer, " error_message={:?}", message.as_str())?;
        }

        writeln!(writer)
    }
}

// Collects the native rejection fields before their redacted formatting.
struct RejectionFields {
    // Native numeric HTTP rejection status.
    status: Option<u64>,
    // Concrete rejection type used to classify transport failures.
    error_variant: Option<String>,
    // Raw diagnostic supplied by the extractor.
    body: Option<String>,
}

impl Visit for RejectionFields {
    // Records the native numeric HTTP rejection status.
    fn record_u64(&mut self, field: &Field, value: u64) {
        //
        if field.name() == "status" {
            self.status = Some(value);
        }
    }

    // Records the concrete rejection type and its raw diagnostic.
    fn record_str(&mut self, field: &Field, value: &str) {
        //
        match field.name() {
            //
            "rejection_type" => self.error_variant = Some(value.to_owned()),

            "body" => self.body = Some(value.to_owned()),

            _ => {}
        }
    }

    // Drops all Debug fields without invoking their formatter.
    fn record_debug(&mut self, _: &Field, _: &dyn Debug) {}
}

// Removes quoted client values while preserving schema-field diagnostics.
fn redact_rejection_message(message: &str) -> String {
    //
    // These forms may interpolate unescaped client text, including delimiters.
    let sensitive = [
        "unknown variant ",
        "unknown field ",
        "invalid type: string ",
        "invalid value: string ",
        "Cannot parse ",
    ]
    .into_iter()
    .filter_map(|marker| message.find(marker).map(|index| (index, marker)))
    .min_by_key(|(index, _)| *index);

    if let Some((index, marker)) = sensitive {
        //
        let prefix = message.get(..index).unwrap_or_default();

        // Preserve serde's source location while dropping the interpolated value.
        let location = message
            .rfind(" at line ")
            .and_then(|offset| message.get(offset..))
            .unwrap_or_default();

        let mut redacted = String::with_capacity(
            prefix.len() + marker.len() + location.len() + 12,
        );

        redacted.push_str(prefix);

        redacted.push_str(marker);

        redacted.push_str("<REDACTED>");

        redacted.push_str(location);

        return redacted;
    }

    let mut safe = String::new();

    let mut chars = message.chars();

    while let Some(character) = chars.next() {
        //
        if !matches!(character, '"' | '`') {
            //
            safe.push(character);

            continue;
        }

        let preserve = character == '`'
            && (safe.ends_with("missing field ")
                || safe.ends_with("duplicate field ")
                || safe.ends_with("Expected request with "));

        safe.push(character);

        if !preserve {
            safe.push_str("<REDACTED>");
        }

        while let Some(quoted) = chars.next() {
            //
            if quoted == character {
                //
                safe.push(quoted);

                break;
            }

            if preserve {
                safe.push(quoted);
            }

            if quoted == '\\' {
                let _ = chars.next();
            }
        }
    }

    safe
}

// Redacts client values and omits transport details from rejection diagnostics.
fn rejection_message(error_variant: Option<&str>, body: &str) -> String {
    //
    let Some(error_variant) = error_variant else {
        return redact_rejection_message(body);
    };

    if !error_variant.contains("UnknownBodyError") {
        return redact_rejection_message(body);
    }

    let Some((prefix, _)) = body.split_once(':') else {
        return body.to_owned();
    };

    prefix.to_owned()
}
