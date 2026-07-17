//! HTTP request handlers grouped by resource.

/// Announcement request handlers.
pub mod announcement;

/// Assignment request handlers.
pub mod assignment;

/// Assignment invitation request handlers.
pub mod assignment_invitation;

/// Authentication request handlers.
pub mod auth;

/// Chapter request handlers.
pub mod chapter;

/// Chapter port request handlers.
pub mod chapter_port;

/// Comic request handlers.
pub mod comic;

/// Comment request handlers.
pub mod comment;

/// Health check request handlers.
pub mod health;

/// Member request handlers.
pub mod member;

/// Member invitation request handlers.
pub mod member_invitation;

/// Page request handlers.
pub mod page;

/// System mail request handlers.
pub mod system_mail;

/// Team request handlers.
pub mod team;

/// Term request handlers.
pub mod term;

/// Termbase request handlers.
pub mod termbase;

/// Unit request handlers.
pub mod unit;

/// User request handlers.
pub mod user;

/// Utility request handlers.
pub mod util;

/// Workset request handlers.
pub mod workset;
