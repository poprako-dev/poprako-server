use async_trait::async_trait;
use time::OffsetDateTime;

use crate::domain::model::aggregate::system_mail::{SystemMailAggr, SystemMailForm};
use crate::domain::query::system_mail::SystemMailQuery;
use crate::domain::result::DomainResult;
use crate::infrastructure::query::memory_mock::MemoryMockQuery;

#[async_trait]
impl SystemMailQuery for MemoryMockQuery {
    async fn send(&self, form: &SystemMailForm) -> DomainResult<()> {
        let mail = SystemMailAggr::new(
            form.id.clone(),
            form.receiver_id.clone(),
            false, // read
            form.title.clone(),
            form.content.clone(),
            OffsetDateTime::now_utc(),
        );

        let mut state = self.state.lock().unwrap();
        state.system_mails.push(mail);

        Ok(())
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::model::aggregate::system_mail::SystemMailForm;

    #[tokio::test]
    async fn send_saves_aggregate() {
        let mock = MemoryMockQuery::new();
        let form = SystemMailForm::new("user-1".into(), "Hello".into(), "Body".into());

        SystemMailQuery::send(&mock, &form).await.unwrap();

        let snap = mock.snapshot();
        assert_eq!(snap.system_mails.len(), 1);
        assert_eq!(snap.system_mails[0].receiver_id, "user-1");
        assert_eq!(snap.system_mails[0].title, "Hello");
        assert_eq!(snap.system_mails[0].content, "Body");
        assert!(!snap.system_mails[0].read);
    }
}
