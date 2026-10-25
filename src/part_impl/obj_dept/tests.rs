//! Object-department regression tests and shared test adapters.

mod identity_tests;

#[cfg(all(feature = "rdb", feature = "repo_impl"))]
mod physical_tests;

#[cfg(feature = "rdb")]
pub mod rdb;

pub mod mock_impl;
