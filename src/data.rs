//! Data transfer objects that sit between the external world and use cases.
//!
//! Request instructions live under [`instr`], direct non-`Info` response values
//! under [`val`], and response views under [`view`]. Every direct projection
//! of a model `*Info` is an `*InfoView`, including list elements. Timestamps
//! are converted to Unix milliseconds, and image keys are resolved through
//! [`ImagePool`].
//!
//! [`ImagePool`]: crate::part::image::ImagePool

/// Request instruction DTOs grouped by domain.
pub mod instr;
/// Direct response value DTOs grouped by domain.
pub mod val;
/// Nested response view DTOs grouped by domain.
pub mod view;
