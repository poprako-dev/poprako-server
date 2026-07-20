//! Includes module declarations and re-exports from the framework sub-module.
//!
//! Concrete per-entity include implementations live in child modules.
//! Framework types (traits, generic engine, `*ByIds` loaders) live in
//! [`framework`].

mod framework;

/// Include logic for announcements.
pub mod announcement;
/// Include logic for assignments.
pub mod assignment;
/// Include logic for chapters.
pub mod chapter;
/// Include logic for comics.
pub mod comic;
/// Include logic for comments.
pub mod comment;
/// Include logic for members.
pub mod member;
/// Include logic for member invitations.
pub mod member_invitation;
