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

pub mod assignment;
pub mod chapter;
pub mod comic;
pub mod member;
pub mod member_invitation;
pub mod page;
pub mod system_mail;
pub mod team;
pub mod unit;
pub mod user;
pub mod workset;
