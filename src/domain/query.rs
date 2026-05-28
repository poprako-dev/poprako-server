pub mod member;
pub mod member_invitation;
pub mod user;

use async_trait::async_trait;
use futures_util::future::BoxFuture;

use crate::domain::query::member::MemberQueryMut;
use crate::domain::query::member_invitation::MemberInvitationQueryMut;
use crate::domain::query::user::UserQeuryMut;
use crate::domain::result::DomainResl;

/// Composite of all mutable query traits required inside a transaction.
///
/// Must be `Send` because it is boxed inside [`Transactional::run_in_transaction`]
/// and passed across `.await` boundaries on a multi-threaded Tokio runtime.
pub trait TransactionalQuery:
    Send + UserQeuryMut + MemberQueryMut + MemberInvitationQueryMut
{
}

/// Transaction controller that provides ACID guarantees for multi-aggregate writes.
///
/// The associated [`Query`](Transactional::Query) type injects a transaction-scoped
/// connection into every mutable repository trait.
#[async_trait]
pub trait Transactional {
    /// Provider that yields mutable queries with an active transaction context.
    type Query<'a>: TransactionalQuery + 'a
    where
        Self: 'a;

    /// Runs `f` inside a database transaction.
    ///
    /// If `f` returns `Ok`, the transaction is committed. If `f` returns `Err`,
    /// the transaction is rolled back. The closure is boxed (→ `BoxFuture`) so it
    /// can cross `.await` boundaries on a multi-threaded Tokio runtime.
    async fn run_in_transaction<F, T>(&self, f: F) -> DomainResl<T>
    where
        T: Send,
        F: for<'a> FnOnce(&'a mut Self::Query<'a>) -> BoxFuture<'a, DomainResl<T>> + Send;
}
