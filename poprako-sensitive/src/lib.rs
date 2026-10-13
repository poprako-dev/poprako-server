//! Declarative Debug redaction and explicit formatting of sensitive values.
//!
//! Values are visible with debug assertions enabled and replaced by
//! `[REDACTED]` otherwise. Serialization and stored values are unchanged.

// Explicit formatting for values without a source-declared sensitive field.
mod sensitive;

pub use poprako_sensitive_macro::SensitiveDebug;

pub use crate::sensitive::Sensitive;
