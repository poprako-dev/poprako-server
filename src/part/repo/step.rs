//! Step definitions — value objects that describe queries and commands
//! to the repository layer.
//!
//! Each step struct implements [`Step`] with an associated [`Output`] type.
//! Steps carry borrowed references to their input data, making them
//! lightweight descriptors rather than owning containers. Use the factory
//! structs ([`UserStep`], [`TeamStep`], etc.) to construct them.
//!
//! [`Step`]: poprako_transactional::step::Step
//! [`Output`]: poprako_transactional::step::Step::Output
//! [`UserStep`]: crate::part::repo::step::user::UserStep
//! [`TeamStep`]: crate::part::repo::step::team::TeamStep

/// Step definitions for the announcement domain.
pub mod announcement;
/// Step definitions for the assignment domain.
pub mod assignment;
/// Step definitions for the assignment invitation domain.
pub mod assignment_invitation;
/// Step definitions for the chapter domain.
pub mod chapter;
/// Step definitions for the comic domain.
pub mod comic;
/// Step definitions for the comment domain.
pub mod comment;
/// Step definitions for the member domain.
pub mod member;
/// Step definitions for the member invitation domain.
pub mod member_invitation;
/// Step definitions for the page domain.
pub mod page;
/// Step definitions for the system mail domain.
pub mod system_mail;
/// Step definitions for the team domain.
pub mod team;
/// Step definitions for the unit domain.
pub mod unit;
/// Step definitions for the user domain.
pub mod user;
/// Step definitions for the workset domain.
pub mod workset;
