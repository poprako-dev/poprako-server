use async_trait::async_trait;

use crate::domain::model::aggregate::system_mail::SystemMailForm;
use crate::domain::result::DomainResult;
use crate::util::ForwardRef;

/// Forwarding marker for [`SystemMailQuery`].
pub struct SystemMailQueryForward;

/// Persistence contract for system mail delivery.
#[async_trait]
pub trait SystemMailQuery {
    /// Sends a system mail notification by inserting a row into `t_system_mail`.
    async fn send(&self, form: &SystemMailForm) -> DomainResult<()>;
}

#[async_trait]
impl<T> SystemMailQuery for T
where
    T: ForwardRef<SystemMailQueryForward> + Sync,
    T::Target: SystemMailQuery + Sync,
{
    async fn send(&self, form: &SystemMailForm) -> DomainResult<()> {
        self.forward_ref().send(form).await
    }
}
