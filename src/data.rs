//! Data transfer objects that sit between the external world and use cases.
//!
//! Types suffixed with `Data` carry inbound request payloads. Types suffixed
//! with `Val` carry presentation-ready outbound values — timestamps are
//! converted to Unix milliseconds, and avatar URLs are resolved through
//! [`ImagePool`].
//!
//! [`ImagePool`]: crate::part::image::ImagePool

pub mod auth;
pub mod team;
pub mod user;
