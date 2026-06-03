pub mod time {
    use time::OffsetDateTime;

    /// Converts a timestamp to Unix epoch milliseconds.
    ///
    /// Implemented for [`OffsetDateTime`](time::OffsetDateTime) to produce
    /// the integer millisecond representation expected by frontend clients.
    pub trait ToUnixMilli {
        /// Returns the number of milliseconds since 1970-01-01T00:00:00Z.
        fn to_unix_milli(&self) -> i64;
    }

    impl ToUnixMilli for OffsetDateTime {
        fn to_unix_milli(&self) -> i64 {
            self.unix_timestamp() * 1000 + (self.nanosecond() / 1_000_000) as i64
        }
    }
}

pub mod err {
    use std::fmt::Debug;
    use std::fmt::Display;

    use crate::util::rename::StdResult;

    /// Emits a [`tracing`] event when a `Result` is `Err`, then passes the
    /// result through unchanged.
    ///
    /// Implemented blanket for [`StdResl<T, E>`](crate::util::rename::StdResl)
    /// where `E: Debug + Display`.
    ///
    /// | Method | Level | Typical use |
    /// |--------|-------|-------------|
    /// | [`trace_debug`](ErrorTrace::trace_debug) | `DEBUG` | Expected/user-facing errors |
    /// | [`trace_info`](ErrorTrace::trace_info) | `INFO` | Informational events |
    /// | [`trace_error`](ErrorTrace::trace_error) | `ERROR` | Unrecoverable/internal errors |
    pub trait ErrorTrace {
        /// Logs `Err` at `DEBUG` level. Use for expected business errors.
        fn trace_debug(self) -> Self;

        /// Logs `Err` at `ERROR` level. Use for unrecoverable internal failures.
        fn trace_error(self) -> Self;
    }

    impl<T, E> ErrorTrace for StdResult<T, E>
    where
        E: Debug + Display,
    {
        fn trace_debug(self) -> Self {
            if let Err(e) = &self {
                tracing::debug!("[trace_debug] {}", e);
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
    pub type StdResult<T, E> = std::result::Result<T, E>;
}

/// Abstracts marker-specific access to an inner type behind a wrapper.
///
/// The marker parameter lets one wrapper forward different trait families to
/// different fields without exposing those fields directly.
pub trait ForwardRef<M> {
    /// The inner type this value forwards to for marker `M`.
    type Target: ?Sized;

    /// Returns a shared reference to the target selected by marker `M`.
    fn forward_ref(&self) -> &Self::Target;
}

/// Implements [`ForwardRef`] for one or more marker types using the same field.
#[macro_export]
macro_rules! impl_forward_ref {
    ($source:ty => $target:ty, $field:ident, $($marker:ty),+ $(,)?) => {
        $(
            impl $crate::util::ForwardRef<$marker> for $source {
                type Target = $target;

                fn forward_ref(&self) -> &$target {
                    &self.$field
                }
            }
        )+
    };
}

pub mod i18n {
    use std::borrow::Cow;
    use std::collections::HashMap;
    use std::sync::LazyLock;

    use fluent_templates::fluent_bundle::FluentValue;
    use fluent_templates::{Loader as _, static_loader};
    use unic_langid::{LanguageIdentifier, langid};

    static_loader! {
        static LOCALES = {
            locales: "locales",
            fallback_language: "zh-CN",
        };
    }

    static LANGUAGE: LazyLock<LanguageIdentifier> = LazyLock::new(|| {
        let language = std::env::var("LANGUAGE").unwrap_or_else(|_| "zh-CN".to_string());
        language.parse().unwrap_or_else(|_| langid!("zh-CN"))
    });

    pub fn trl(key: &str) -> String {
        LOCALES.lookup(&LANGUAGE, key)
    }

    /// Looks up a parameterized Fluent message for the current language.
    ///
    /// `args` maps Fluent variable names (without the `$` prefix) to their
    /// [`FluentValue`] replacements.  Variables in the `.ftl` file must use
    /// the `{$name}` syntax.
    pub fn trl_kv(key: &str, args: &HashMap<Cow<'static, str>, FluentValue>) -> String {
        LOCALES.lookup_with_args(&LANGUAGE, key, args)
    }
}
