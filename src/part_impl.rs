/// Authentication port implementation (JWT signing, mock).
pub mod auth;
/// Transaction driver implementation (RDBMS-based).
pub mod drive;
// TODO:
/// Effect processing implementation (async dispatch, mock).
pub mod effect;
/// Image pool implementation (R2 object storage, mock).
pub mod image;
/// Prom (deferred action) implementation (RDBMS-based, mock).
pub mod prom;
/// Repository implementations (RDBMS, mock).
pub mod repo;
pub mod schd;
/// Shared utilities for part implementations.
pub mod shared;
