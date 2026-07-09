//! Defines the [`Drive`] trait, a transactional context abstraction that
//! allows running fallible steps within a backend-provided scope.
//!
//! The [`result`] submodule pairs this trait with its error types.

pub mod result;

use async_trait::async_trait;

use crate::drive::result::Error as DriveError;
use crate::util::AsyncFnMark;

/// A transactional scope that executes a fallible function within a managed
/// [`C`]ontext, mapping step and backend errors into a unified [`DriveError`].
#[async_trait]
pub trait Drive<C> {
    /// The error type.
    type Error;

    async fn with_context<T, E, F>(
        &self,
        f: F,
    ) -> Result<T, DriveError<E, Self::Error>>
    where
        T: Send,
        E: Send,
        for<'c> F: AsyncFnOnce(&'c mut C) -> Result<T, E>
            + AsyncFnMark<&'c mut C, Result<T, E>, Fut: Send>
            + Send;
}
