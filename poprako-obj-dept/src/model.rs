//! Object lifecycle values split by function.

/// Selected object generations and upload requirements.
pub mod alloc;
/// Current object metadata.
pub mod meta;
/// Upload capability values.
pub mod slot;
/// Durable object-task values.
pub mod task;
/// Resolved object read URLs.
pub mod url;
