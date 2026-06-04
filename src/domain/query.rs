pub mod member;
pub mod member_invitation;
pub mod system_mail;
pub mod team;
pub mod user;

use async_trait::async_trait;
use futures_util::future::BoxFuture;

use crate::domain::query::member::MemberQueryTransactional;
use crate::domain::query::member_invitation::MemberInvitationQueryTransactional;
use crate::domain::query::system_mail::SystemMailQuery;
use crate::domain::query::team::TeamQuery;
use crate::domain::query::user::{UserQuery, UserQueryTransactional};
use crate::domain::result::DomainResult;
use poprako_util::ForwardRef;

/// Composite read-only query contract for non-transactional use cases.
pub trait Query: UserQuery + TeamQuery + SystemMailQuery {}

impl<T> Query for T where T: UserQuery + TeamQuery + SystemMailQuery {}

/// Forwarding marker for [`Transactional`].
pub struct TransactionalForward;

/// Composite of all mutable query traits required inside a transaction.
///
/// Must be `Send` because it is boxed inside [`Transactional::run_in_transaction`]
/// and passed across `.await` boundaries on a multi-threaded Tokio runtime.
pub trait QueryTransactional:
    UserQueryTransactional + MemberQueryTransactional + MemberInvitationQueryTransactional
{
}

impl<T> QueryTransactional for T where
    T: UserQueryTransactional + MemberQueryTransactional + MemberInvitationQueryTransactional
{
}

/// Transaction controller that provides ACID guarantees for multi-aggregate writes.
///
/// The associated [`Query`](Transactional::Query) type injects a transaction-scoped
/// connection into every mutable repository trait.
#[async_trait]
pub trait Transactional {
    /// Provider that yields mutable queries with an active transaction context.
    type Query<'a>: QueryTransactional + Send + 'a
    where
        Self: 'a;

    /// Runs `f` inside a database transaction.
    ///
    /// If `f` returns `Ok`, the transaction is committed. If `f` returns `Err`,
    /// the transaction is rolled back. The closure is boxed (→ `BoxFuture`) so it
    /// can cross `.await` boundaries on a multi-threaded Tokio runtime.
    async fn transaction_scoped<F, T>(&self, f: F) -> DomainResult<T>
    where
        T: Send,
        F: for<'a> FnOnce(&'a mut Self::Query<'a>) -> BoxFuture<'a, DomainResult<T>> + Send;
}

/// Any type whose transaction forwarding target implements [`Transactional`]
/// is itself [`Transactional`], delegating to the selected target.
#[async_trait]
impl<T> Transactional for T
where
    T: ForwardRef<TransactionalForward> + Send + Sync + 'static,
    T::Target: Transactional,
{
    type Query<'a>
        = <T::Target as Transactional>::Query<'a>
    where
        Self: 'a;

    async fn transaction_scoped<F, U>(&self, f: F) -> DomainResult<U>
    where
        U: Send,
        F: for<'a> FnOnce(&'a mut Self::Query<'a>) -> BoxFuture<'a, DomainResult<U>> + Send,
    {
        self.forward_ref().transaction_scoped(f).await
    }
}
