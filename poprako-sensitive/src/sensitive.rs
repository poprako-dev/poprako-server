//! Explicit formatting for values whose source type cannot declare redaction.

use core::fmt::{Debug, Display, Formatter, Result};

/// Wraps an owned or borrowed value for configuration-dependent formatting.
///
/// Debug assertions expose the original Debug or Display representation.
/// Without debug assertions, neither formatter touches the wrapped value.
pub struct Sensitive<T> {
    /// Original value, never formatted in the redacted configuration.
    #[cfg_attr(
        not(debug_assertions),
        allow(
            dead_code,
            reason = "Redacted formatting never reads the stored value"
        )
    )]
    value: T,
}

impl<T> Sensitive<T> {
    /// Wraps a value; pass a reference to avoid moving or cloning it.
    pub const fn new(value: T) -> Self {
        Self { value }
    }
}

impl<T> Debug for Sensitive<T>
where
    T: Debug,
{
    // Preserve the underlying formatter, including alternate Debug formatting.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        //
        #[cfg(debug_assertions)]
        {
            Debug::fmt(&self.value, formatter)
        }

        #[cfg(not(debug_assertions))]
        {
            Debug::fmt(&"[REDACTED]", formatter)
        }
    }
}

impl<T> Display for Sensitive<T>
where
    T: Display,
{
    // Apply the same redaction policy to tracing's percent formatter.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        //
        #[cfg(debug_assertions)]
        {
            Display::fmt(&self.value, formatter)
        }

        #[cfg(not(debug_assertions))]
        {
            formatter.write_str("[REDACTED]")
        }
    }
}
