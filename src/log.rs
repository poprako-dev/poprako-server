/// Initialise the tracing subscriber with sensible defaults for the
/// application binary.
///
/// Reads `RUST_LOG` from the environment and falls back to `INFO` when no
/// directive is set.  Colours are enabled in debug builds only so the
/// release output is plain text suitable for log aggregation.
pub fn init_log() {
    //
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(
                    tracing_subscriber::filter::LevelFilter::INFO.into(),
                )
                .from_env_lossy(),
        )
        .with_ansi(cfg!(debug_assertions))
        .init();
}
