//! Data transfer objects that sit between the external world and use cases.
//!
//! Request instructions live under [`instr`], direct response values under
//! [`val`], and response-only nested views under [`view`]. Timestamps are
//! converted to Unix milliseconds, and image keys are resolved through
//! [`ImagePool`].
//!
//! [`ImagePool`]: crate::part::image::ImagePool

/// Request instruction DTOs grouped by domain.
pub mod instr;
/// Direct response value DTOs grouped by domain.
pub mod val;
/// Nested response view DTOs grouped by domain.
pub mod view;
