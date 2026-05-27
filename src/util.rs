pub mod time {
    use time::OffsetDateTime;

    pub trait ToUnixMilli {
        fn to_unix_milli(&self) -> i64;
    }

    impl ToUnixMilli for OffsetDateTime {
        fn to_unix_milli(&self) -> i64 {
            self.unix_timestamp() * 1000 + (self.nanosecond() / 1_000_000) as i64
        }
    }
}

pub mod err {
    use crate::util::rename::StdResl;

    pub trait ErrorTrace {
        // Output a tracing message when a debug is generated.
        fn trace_debug(self) -> Self;

        // Output a tracing message when an info is generated.
        fn trace_info(self) -> Self;

        // Output a tracing message when a error is generated.
        fn trace_error(self) -> Self;
    }

    impl<T, E> ErrorTrace for StdResl<T, E>
    where
        E: std::fmt::Debug + std::fmt::Display,
    {
        fn trace_debug(self) -> Self {
            if let Err(e) = &self {
                tracing::debug!("[trace_debug] {}", e);
            }
            self
        }

        fn trace_info(self) -> Self {
            if let Err(e) = &self {
                tracing::info!("[trace_info] {}", e);
            }
            self
        }

        fn trace_error(self) -> Self {
            if let Err(e) = &self {
                tracing::error!("[trace_error] {}", e);
            }
            self
        }
    }
}

pub mod rename {
    pub type StdResl<T, E> = std::result::Result<T, E>;
}
