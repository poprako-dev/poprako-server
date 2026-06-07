use async_trait::async_trait;

use poprako_util::i18n::trl;

use crate::domain::model::aggr::team::TeamAggr;
use crate::domain::query::team::TeamQuery;
use crate::domain::result::{DomainError, DomainResult};
use crate::infra::query::memory_mock::MemoryMockQuery;

#[async_trait]
impl TeamQuery for MemoryMockQuery {
    async fn get_by_id(&self, id: &str) -> DomainResult<TeamAggr> {
        let state = self.state.lock().unwrap();
        state
            .teams
            .iter()
            .find(|t| t.id == id)
            .cloned()
            .ok_or_else(|| DomainError::expected_argument(trl("error-team-not-found")))
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    // find_by_id_after_seed(TeamQuery::get_by_id)(positive): seeded teams should be found by ID.
    // get_by_id_missing_returns_expected_error(TeamQuery::get_by_id)(negative): missing teams should return an expected argument error.

    use time::OffsetDateTime;

    use crate::domain::model::aggr::team::TeamAggr;
    use crate::domain::query::team::TeamQuery;
    use crate::infra::query::memory_mock::MemoryMockQuery;
    use crate::test_util::is_expected_argument;

    fn now() -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }

    fn make_team(id: &str) -> TeamAggr {
        let n = now();
        TeamAggr {
            id: id.into(),
            name: "team-name".into(),
            description: "desc".into(),
            avatar_key: String::new(),
            avatar_uploaded: false,
            created_at: n,
            updated_at: n,
        }
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
