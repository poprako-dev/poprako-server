//! Production logging with structured, payload-free Axum rejection diagnostics.

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
/// Rejection payloads are excluded from output, even with broader TRACE logging.
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
    // Formats native rejection events without visiting private payload values.
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

        write!(writer, " {} ", event.metadata().level())?;

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
            writer,
            format_rest: Ok(()),
        };

        event.record(&mut fields);

        fields.format_rest?;

        writeln!(fields.writer)
    }
}

// Never format body, message, or unknown fields, including through Debug.
struct RejectionFields<'a> {
    // Output stream borrowed for this rejection event.
    writer: Writer<'a>,
    // First formatting failure, if any.
    format_rest: FmtResult,
}

impl Visit for RejectionFields<'_> {
    // Records the native numeric HTTP rejection status.
    fn record_u64(&mut self, field: &Field, value: u64) {
        //
        if self.format_rest.is_err() || field.name() != "status" {
            return;
        }

        self.format_rest = self
            .writer
            .write_str(" status=")
            .and_then(|()| self.writer.write_str(&value.to_string()));
    }

    // Records the library-controlled concrete rejection type.
    fn record_str(&mut self, field: &Field, value: &str) {
        //
        if self.format_rest.is_err() || field.name() != "rejection_type" {
            return;
        }

        self.format_rest =
            self.writer.write_str(" error_variant=").and_then(|()| {
                //
                self.writer
                    .write_str(&format!("{:?}", tracing::field::debug(value)))
            });
    }

    // Drops all Debug fields without invoking their formatter.
    fn record_debug(&mut self, _: &Field, _: &dyn Debug) {}
}
