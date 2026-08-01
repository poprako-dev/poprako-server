/// Authentication port implementation (JWT signing, mock).
pub mod auth;
/// Effect processing implementation (async dispatch, mock).
pub mod effect;
/// Image pool implementation (R2 object storage, mock).
pub mod image;
/// Transaction coordinator implementation (RDBMS-based).
pub mod nucl;
/// Prom (deferred action) implementation (RDBMS-based, mock).
pub mod prom;
/// Repository implementations (RDBMS, mock).
pub mod repo;
