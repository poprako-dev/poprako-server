pub mod member;
pub mod member_invitation;
pub mod user;

use async_trait::async_trait;
use futures_util::future::BoxFuture;

use crate::domain::query::member::MemberQueryMut;
use crate::domain::query::member_invitation::MemberInvitationQueryMut;
use crate::domain::query::user::UserQeuryMut;
use crate::domain::result::DomainResl;

pub trait TransactionalQuery:
    Send + UserQeuryMut + MemberQueryMut + MemberInvitationQueryMut
{
}

#[async_trait]
pub trait Transactional: Send + Sync {
    // Context is the provider that gives out queries with transaction context injected.
    type Query<'a>: TransactionalQuery + 'a
    where
        Self: 'a;

    // run_in_transaction runs the given function in a transaction, and returns the result of the function.
    async fn run_in_transaction<F, T>(&self, f: F) -> DomainResl<T>
    where
        T: Send,
        F: for<'a> FnOnce(&'a mut Self::Query<'a>) -> BoxFuture<'a, DomainResl<T>> + Send;
}
