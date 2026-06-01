use async_trait::async_trait;

use crate::domain::model::aggregate::team::TeamAggr;
use crate::domain::query::team::TeamQuery;
use crate::domain::result::{DomainError, DomainResult};
use crate::infrastructure::query::memory_mock::MemoryMockQuery;
use crate::util::err::ErrorTrace as _;
use crate::util::i18n::trl;

#[async_trait]
impl TeamQuery for MemoryMockQuery {
    async fn get_by_id(&self, id: &str) -> DomainResult<TeamAggr> {
        let state = self.state.lock().unwrap();
        state
            .teams
            .iter()
            .find(|t| t.id == id)
            .cloned()
            .ok_or(DomainError::expected_argument(trl("error-team-not-found")))
            .trace_debug()
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::result::{DomainError, ExpectedVariant};
    use time::OffsetDateTime;

    fn now() -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }

    fn make_team(id: &str) -> TeamAggr {
        TeamAggr::new(
            id.into(),
            "team-name".into(),
            "desc".into(),
            String::new(),
            false,
            now(),
            now(),
        )
    }

    fn is_expected_argument(err: &DomainError) -> bool {
        matches!(
            err,
            DomainError::Expected {
                variant: ExpectedVariant::Argument,
                ..
            }
        )
    }

    #[tokio::test]
    async fn find_by_id_after_seed() {
        let mock = MemoryMockQuery::new();
        mock.seed_team(make_team("team-1"));

        let found = TeamQuery::get_by_id(&mock, "team-1").await.unwrap();
        assert_eq!(found.id, "team-1");
    }

    #[tokio::test]
    async fn get_by_id_missing_returns_expected_error() {
        let mock = MemoryMockQuery::new();

        let err = TeamQuery::get_by_id(&mock, "nonexistent")
            .await
            .err()
            .unwrap();
        assert!(is_expected_argument(&err));
    }
}
