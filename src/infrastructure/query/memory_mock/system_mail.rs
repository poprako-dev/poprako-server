use async_trait::async_trait;
use time::OffsetDateTime;

use crate::domain::model::aggregate::system_mail::{SystemMailAggr, SystemMailForm};
use crate::domain::query::system_mail::SystemMailQuery;
use crate::domain::result::DomainResult;
use crate::infrastructure::query::memory_mock::MemoryMockQuery;

#[async_trait]
impl SystemMailQuery for MemoryMockQuery {
    async fn send(&self, form: &SystemMailForm) -> DomainResult<()> {
        let mail = SystemMailAggr {
            id: form.id.clone(),
            receiver_id: form.receiver_id.clone(),
            read: false,
            title: form.title.clone(),
            content: form.content.clone(),
            created_at: OffsetDateTime::now_utc(),
        };

        let mut state = self.state.lock().unwrap();
        state.system_mails.push(mail);

        Ok(())
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    // send_saves_aggregate(SystemMailQuery::send)(positive): sending system mail should save the aggregate in memory.

    use crate::domain::model::aggregate::system_mail::{SystemMailAggr, SystemMailForm};
    use crate::domain::query::system_mail::SystemMailQuery;
    use crate::infrastructure::query::memory_mock::MemoryMockQuery;

    #[tokio::test]
    async fn send_saves_aggregate() {
        let mock = MemoryMockQuery::new();
        let form = SystemMailForm {
            id: SystemMailAggr::generate_id(),
            receiver_id: "user-1".into(),
            title: "Hello".into(),
            content: "Body".into(),
        };

        SystemMailQuery::send(&mock, &form).await.unwrap();

        let snap = mock.snapshot();
        assert_eq!(snap.system_mails.len(), 1);
        assert_eq!(snap.system_mails[0].receiver_id, "user-1");
        assert_eq!(snap.system_mails[0].title, "Hello");
        assert_eq!(snap.system_mails[0].content, "Body");
        assert!(!snap.system_mails[0].read);
    }
}
