//! Application service layer.
//!
//! Each public function in this module orchestrates a single use case by
//! composing the port abstractions from [`part`](super::part). Functions are
//! free-standing (not methods on a struct) and are generic over their
//! dependencies, enabling easy substitution of mock implementations in tests.
//!
//! # Execution model
//!
//! Use cases compose two execution modes:
//!
//! - **Standalone** — independent operations use Orchestra [`Run`].
//!
//! - **Coordinated** — multi-step operations run inside [`Nucl::coord`]. All
//!   [`Step`] calls share one context and commit or roll back atomically.
//!   Side-effects and prom records keep their existing post-commit ordering.
//!
//! # Type parameters
//!
//! Most functions carry several generic type parameters. The common ones are:
//!
//! | Parameter | Role |
//! |-----------|------|
//! | `N: Nucl<Context = C>` | Transaction coordinator |
//! | `C` | Context shared by coordinated steps |
//! | `R: XxxRepo<C>` | Repository bundle for data access |
//! | `P: Prom<C>` | Deferred-action enqueuer |
//! | `I: ImagePool` | Object-storage signed URL provider |
//! | `V: EffectDevelop` | Side-effect processor for domain events |
//! | `A: TokenAuth` | Authentication token signer |
//!
//! [`Nucl::coord`]: poprako_orchestra::Nucl::coord
//! [`Run`]: poprako_orchestra::Run
//! [`Step`]: poprako_orchestra::Step

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
// Stage-processing use cases (internal).
mod stage;
/// System mail use cases.
pub mod system_mail;
/// Team management use cases.
pub mod team;
/// Terminology-entry use cases.
pub mod term;
/// Termbase management use cases.
pub mod termbase;
/// Unit ordering use cases.
pub mod unit;
/// User management use cases.
pub mod user;
/// Workset lifecycle use cases.
pub mod workset;
