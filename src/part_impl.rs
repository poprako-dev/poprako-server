/// Authentication port implementation (JWT signing, mock).
pub mod auth;
/// Effect processing implementation (async dispatch, mock).
pub mod effect;
/// Transaction coordinator implementation (RDBMS-based).
pub mod nucl;
/// Reliable remote-object lifecycle implementation.
pub mod obj_dept;
/// Prom (deferred action) implementation (RDBMS-based, mock).
pub mod prom;
/// Repository implementations (RDBMS, mock).
pub mod repo;
