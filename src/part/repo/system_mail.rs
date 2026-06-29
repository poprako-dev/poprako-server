//! Repository traits for the system mail domain.
//!
//! All system mail opers are non-transactional single-table writes
//! and reads — the transactional [`SystemMailRepoTransactional`] exists
//! only as a type-system anchor to keep the repo trait pattern consistent.

use crate::part::repo::step::system_mail::{
    ListInfosByIds, ListInfosByReceiverId, MarkRead, Send, SendBatch,
};
use crate::part::shared::execute::Execute;
use crate::result::RootError;
use crate::util::DeriveTransactional;

/// Non-transactional system mail repository.
///
/// All system mail opers are simple single-table reads and writes
/// that do not require transactional atomicity.
pub trait SystemMailRepo<C>:
    DeriveTransactional
    + for<'a> Execute<Send<'a>, Error = RootError>
    + for<'a> Execute<SendBatch<'a>, Error = RootError>
    + for<'a> Execute<ListInfosByReceiverId<'a>, Error = RootError>
    + for<'a> Execute<ListInfosByIds<'a>, Error = RootError>
    + for<'a> Execute<MarkRead<'a>, Error = RootError>
where
    Self::Transactional: SystemMailRepoTransactional<C>,
{
}

/// Transactional system mail repository.
///
/// Currently empty — all system mail opers are non-transactional.
/// The trait exists solely as a type-level anchor for the
/// [`SystemMailRepo`] trait pattern.
pub trait SystemMailRepoTransactional<C>: Sized {}
