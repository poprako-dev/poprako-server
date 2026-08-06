//! Repository capabilities implemented with PopRaKo Orchestra.
//!
//! # The `C` type parameter
//!
//! The `C` generic parameter anchors [`Step`] operations to the context
//! supplied by the matching [`Nucl`].
//!
//! This prevents, at compile time, wiring a production repository's
//! transactional handle to a mock context (or vice versa). Within a single
//! use case function scope, only one `C` implementation exists (either real
//! or mock), and the type system resolves the correct path automatically.
//!
//! # Standalone and coordinated operations
//!
//! Operations implemented via [`Run`] are independent. Operations implemented
//! via [`Step`] participate in the caller's [`Nucl::coord`] context and are
//! committed or rolled back together.
//!
//! Each domain exposes one `XxxRepo<C>` capability trait that aggregates its
//! required `Run` and `Step` implementations.
//!
//! [`Nucl`]: poprako_orchestra::Nucl
//! [`Nucl::coord`]: poprako_orchestra::Nucl::coord
//! [`Run`]: poprako_orchestra::Run
//! [`Step`]: poprako_orchestra::Step

/// Announcement repository port.
pub mod announcement;
/// Assignment repository port.
pub mod assignment;
/// Assignment invitation repository port.
pub mod assignment_invitation;
/// Chapter repository port.
pub mod chapter;
/// Comic repository port.
pub mod comic;
/// Immutable comic archive repository port.
pub mod comic_archive;
/// Comment repository port.
pub mod comment;
/// Member repository port.
pub mod member;
/// Member invitation repository port.
pub mod member_invitation;
/// Online-user repository port.
pub mod online_user;
/// Repository operation descriptors.
pub mod oper;
/// Page repository port.
pub mod page;
/// System mail repository port.
pub mod system_mail;
/// Team repository port.
pub mod team;
/// Term repository port.
pub mod term;
/// Termbase repository port.
pub mod termbase;
/// Unit repository port.
pub mod unit;
/// User repository port.
pub mod user;
/// Workset repository port.
pub mod workset;
