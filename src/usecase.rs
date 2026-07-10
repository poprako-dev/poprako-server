//! Application service layer.
//!
//! Each public function in this module orchestrates a single use case by
//! composing the port abstractions from [`part`](super::part). Functions are
//! free-standing (not methods on a struct) and are generic over their
//! dependencies, enabling easy substitution of mock implementations in tests.
//!
//! # Execution model
//!
//! Use cases fall into two categories:
//!
//! - **Non-transactional** — simple reads or single-row writes that call
//!   [`repo.execute(...)`](Execute::execute) directly. Each call uses its own
//!   database connection and commits independently.
//!
//! - **Transactional** — multi-step opers wrapped in
//!   [`drive.with_context(...)`](Drive::with_context). All [`Advance`] calls
//!   within the block share a transaction and commit or rollback atomically.
//!   Side-effects (events, prom records) are deferred until after commit.
//!
//! # Type parameters
//!
//! Most functions carry several generic type parameters. The common ones are:
//!
//! | Parameter | Role |
//! |-----------|------|
//! | `D: Drive<C>` | Transaction lifecycle driver |
//! | `C` | Context anchor linking transactional opers |
//! | `R: XxxRepo<C>` | Repository bundle for data access |
//! | `P: Prom<C>` | Deferred-action enqueuer |
//! | `I: ImagePool` | Object-storage signed URL provider |
//! | `V: EffectDevelop` | Side-effect processor for domain events |
//! | `A: TokenAuth` | Authentication token signer |
//!
//! [`Execute::execute`]: crate::part::shared::execute::Execute::execute
//! [`Drive::with_context`]: poprako_transactional::drive::Drive::with_context
//! [`Advance`]: poprako_transactional::advance::Advance

/// Announcement use cases.
pub mod announcement;
/// Assignment management use cases.
pub mod assignment;
/// Assignment invitation use cases.
pub mod assignment_invitation;
/// Authentication use cases (register, login, logout).
pub mod auth;
/// Chapter lifecycle use cases.
pub mod chapter;
/// Chapter port import/export use cases.
pub mod chapter_port;
/// Comic lifecycle use cases.
pub mod comic;
/// Immutable comic archive use cases.
pub mod comic_archive;
/// Comment use cases.
pub mod comment;
/// Member management use cases.
pub mod member;
/// Member invitation use cases.
pub mod member_invitation;
/// Page management use cases.
pub mod page;
/// System mail use cases.
pub mod system_mail;
/// Team management use cases.
pub mod team;
/// Unit ordering use cases.
pub mod unit;
/// User management use cases.
pub mod user;
/// Workset lifecycle use cases.
pub mod workset;
