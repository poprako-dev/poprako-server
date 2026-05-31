use async_trait::async_trait;

use crate::domain::model::aggregate::system_mail::SystemMailForm;
use crate::domain::result::DomainResult;

/// Read-only persistence contract for system mail delivery.
///
/// Each method takes an immutable `&self` reference, suitable for
/// non-transactional queries backed by a connection pool.
#[async_trait]
pub trait SystemMailQuery {
    /// Sends a system mail notification by inserting a row into `t_system_mail`.
    async fn send(&self, form: SystemMailForm) -> DomainResult<()>;
}
