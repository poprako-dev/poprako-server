//! Includes module declarations and private declarative macros.

#[macro_use]
mod macros;
// Framework include helper traits.
mod framework;

// Generic batch-include framework with per-table loaders.
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

#[cfg(test)]
mod tests;
