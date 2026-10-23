//! Domain models representing business entities and their read and write forms.
//!
//! Models own their stored data; methods may expose borrowed views.
//! Performance-driven borrowed payloads live in the sibling `shape` layer.
//! Models follow business meaning and need not map one-to-one to persisted rows.
//! Read models describe queried state, write models describe intended changes,
//! and shared models carry concepts used across both roles. Presentation types
//! belong to the [`data`](super::data) layer.

/// Business read models and query specifications.
pub mod read;
/// Business models shared by read and write flows.
pub mod shared;
/// Business write models, including imports and intended mutations.
pub mod write;

#[cfg(test)]
mod tests;
