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

pub mod auth;
pub mod user;

// TODO: check effects.
pub mod system_mail;

pub mod member;
pub mod member_invitation;
pub mod team;

pub mod chapter;
pub mod comic;
pub mod workset;

pub mod assignment;
pub mod assignment_invitation;

pub mod page;
pub mod unit;

pub mod announcement;
pub mod comment;

pub mod chapter_port;
