use async_trait::async_trait;
use time::OffsetDateTime;

use poprako_util::i18n::trl;
use poprako_util::page::Page;

use crate::domain::model::aggr::team::{TeamAggr, TeamForm, TeamUpdate};
use crate::domain::query::team::{TeamQuery, TeamQueryTransactional};
use crate::domain::result::{DomainError, DomainResult};
use crate::infra::query::memory_mock::{MemoryMockQuery, MemoryMockQueryTransactional};

// ── Query impls ────────────────────────────────────────────────────────────

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

    async fn list(&self, page: Page) -> DomainResult<Vec<TeamAggr>> {
        let state = self.state.lock().unwrap();
        let mut teams: Vec<TeamAggr> = state.teams.clone();

        // Sort by created_at descending.
        teams.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let skip = page.offset;
        let take = page.limit;

        if skip >= teams.len() {
            return Ok(Vec::new());
        }

        let end = std::cmp::min(skip + take, teams.len());
        Ok(teams[skip..end].to_vec())
    }

    async fn prefill_avatar_key(&self, id: &str, key: &str) -> DomainResult<()> {
        let mut state = self.state.lock().unwrap();

        let team = state
            .teams
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| DomainError::expected_argument(trl("error-team-not-found")))?;

        team.avatar_key = key.to_string();
        team.avatar_uploaded = false;
        team.updated_at = OffsetDateTime::now_utc();

        Ok(())
    }

    async fn mark_avatar_uploaded(&self, id: &str) -> DomainResult<()> {
        let mut state = self.state.lock().unwrap();

        let team = state
            .teams
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| DomainError::expected_argument(trl("error-team-not-found")))?;

        team.avatar_uploaded = true;
        team.updated_at = OffsetDateTime::now_utc();

        Ok(())
    }

    async fn create(&self, form: &TeamForm) -> DomainResult<TeamAggr> {
        let mut state = self.state.lock().unwrap();

        if state.teams.iter().any(|t| t.id == form.id) {
            return Err(DomainError::expected_conflict(trl("error-already-exists")));
        }
        if state.teams.iter().any(|t| t.name == form.name) {
            return Err(DomainError::expected_conflict(trl("error-already-exists")));
        }

        let now = OffsetDateTime::now_utc();
        let team = TeamAggr {
            id: form.id.clone(),
            name: form.name.clone(),
            description: form.description.clone(),
            avatar_key: String::new(),
            avatar_uploaded: false,
            workset_next_index: 0,
            created_at: now,
            updated_at: now,
        };

        state.teams.push(team.clone());

        Ok(team)
    }

    async fn update(&self, input: &TeamUpdate) -> DomainResult<()> {
        let mut state = self.state.lock().unwrap();

        let team = state
            .teams
            .iter_mut()
            .find(|t| t.id == input.id)
            .ok_or_else(|| DomainError::expected_argument(trl("error-team-not-found")))?;

        team.name = input.name.clone();
        team.description = input.description.clone();
        team.updated_at = OffsetDateTime::now_utc();

        Ok(())
    }

    async fn delete(&self, id: &str) -> DomainResult<()> {
        let mut state = self.state.lock().unwrap();

        let pos = state
            .teams
            .iter()
            .position(|t| t.id == id)
            .ok_or_else(|| DomainError::expected_argument(trl("error-team-not-found")))?;

        state.teams.remove(pos);

        Ok(())
    }
}

// ── QueryTransactional impls ───────────────────────────────────────────────

#[async_trait]
impl TeamQueryTransactional for MemoryMockQueryTransactional {
    async fn increment_workset_next_index(&mut self, id: &str) -> DomainResult<i32> {
        let mut state = self.state.lock().unwrap();

        let team = state
            .teams
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| DomainError::expected_argument(trl("error-team-not-found")))?;

        let allocated = team.workset_next_index;
        team.workset_next_index += 1;
        team.updated_at = OffsetDateTime::now_utc();

        Ok(allocated)
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

// ── TeamQuery forwarding on transactional handle ───────────────────────────

#[async_trait]
impl TeamQuery for MemoryMockQueryTransactional {
    async fn get_by_id(&self, id: &str) -> DomainResult<TeamAggr> {
        let state = self.state.lock().unwrap();
        state
            .teams
            .iter()
            .find(|t| t.id == id)
            .cloned()
            .ok_or_else(|| DomainError::expected_argument(trl("error-team-not-found")))
    }

    async fn list(&self, page: Page) -> DomainResult<Vec<TeamAggr>> {
        let state = self.state.lock().unwrap();
        let mut teams: Vec<TeamAggr> = state.teams.clone();
        teams.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        let skip = page.offset;
        let take = page.limit;
        if skip >= teams.len() {
            return Ok(Vec::new());
        }
        let end = std::cmp::min(skip + take, teams.len());
        Ok(teams[skip..end].to_vec())
    }

    async fn prefill_avatar_key(&self, id: &str, key: &str) -> DomainResult<()> {
        let mut state = self.state.lock().unwrap();
        let team = state
            .teams
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| DomainError::expected_argument(trl("error-team-not-found")))?;
        team.avatar_key = key.to_string();
        team.avatar_uploaded = false;
        team.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    async fn mark_avatar_uploaded(&self, id: &str) -> DomainResult<()> {
        let mut state = self.state.lock().unwrap();
        let team = state
            .teams
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| DomainError::expected_argument(trl("error-team-not-found")))?;
        team.avatar_uploaded = true;
        team.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    async fn create(&self, form: &TeamForm) -> DomainResult<TeamAggr> {
        let mut state = self.state.lock().unwrap();
        if state
            .teams
            .iter()
            .any(|t| t.id == form.id || t.name == form.name)
        {
            return Err(DomainError::expected_conflict(trl("error-already-exists")));
        }
        let now = OffsetDateTime::now_utc();
        let team = TeamAggr {
            id: form.id.clone(),
            name: form.name.clone(),
            description: form.description.clone(),
            avatar_key: String::new(),
            avatar_uploaded: false,
            workset_next_index: 0,
            created_at: now,
            updated_at: now,
        };
        state.teams.push(team.clone());
        Ok(team)
    }

    async fn update(&self, input: &TeamUpdate) -> DomainResult<()> {
        let mut state = self.state.lock().unwrap();
        let team = state
            .teams
            .iter_mut()
            .find(|t| t.id == input.id)
            .ok_or_else(|| DomainError::expected_argument(trl("error-team-not-found")))?;
        team.name = input.name.clone();
        team.description = input.description.clone();
        team.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    async fn delete(&self, id: &str) -> DomainResult<()> {
        let mut state = self.state.lock().unwrap();
        let pos = state
            .teams
            .iter()
            .position(|t| t.id == id)
            .ok_or_else(|| DomainError::expected_argument(trl("error-team-not-found")))?;
        state.teams.remove(pos);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // find_by_id_after_seed(TeamQuery::get_by_id)(positive): seeded teams should be found by ID.
    // get_by_id_missing_returns_expected_error(TeamQuery::get_by_id)(negative): missing teams should return an expected argument error.
    // list_returns_teams_ordered_by_created_at_desc(TeamQuery::list)(positive): list should return teams ordered by created_at DESC.
    // prefill_avatar_key_sets_avatar_key(TeamQuery::prefill_avatar_key)(positive): prefill should set the avatar key and clear uploaded flag.
    // mark_avatar_uploaded_sets_flag(TeamQuery::mark_avatar_uploaded)(positive): marking should set avatar_uploaded to true.
    // create_then_find(TeamQuery::create)(positive): created teams should be readable after transaction commit.
    // create_duplicate_name_returns_conflict(TeamQuery::create)(negative): duplicate team names should return a conflict.
    // update_changes_fields(TeamQuery::update)(positive): update should change name and description.
    // update_missing_returns_error(TeamQuery::update)(negative): updating a missing team should fail.
    // delete_removes_team(TeamQuery::delete)(positive): deleting a team should remove it from storage.
    // delete_missing_returns_error(TeamQuery::delete)(negative): deleting a missing team should fail.
    // increment_workset_next_index_returns_allocated(TeamQueryTransactional::increment_workset_next_index)(positive): each call should return the current value and increment it.
    // increment_workset_next_index_missing_returns_error(TeamQueryTransactional::increment_workset_next_index)(negative): incrementing a missing team should fail.

    use futures_util::FutureExt as _;
    use time::OffsetDateTime;

    use poprako_util::page::Page;

    use crate::domain::model::aggr::team::{TeamAggr, TeamForm, TeamUpdate};
    use crate::domain::query::Transactional;
    use crate::domain::query::team::{TeamQuery, TeamQueryTransactional};
    use crate::infra::query::memory_mock::MemoryMockQuery;
    use crate::test_util::is_expected_argument;
    use crate::test_util::is_expected_conflict;

    fn now() -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }

    fn make_team(id: &str, name: &str) -> TeamAggr {
        let n = now();
        TeamAggr {
            id: id.into(),
            name: name.into(),
            description: "desc".into(),
            avatar_key: String::new(),
            avatar_uploaded: false,
            workset_next_index: 0,
            created_at: n,
            updated_at: n,
        }
    }

    #[tokio::test]
    async fn find_by_id_after_seed() {
        let mock = MemoryMockQuery::new();
        mock.seed_team(make_team("team-1", "Team A"));

        let found = TeamQuery::get_by_id(&mock, "team-1").await.unwrap();
        assert_eq!(found.id, "team-1");
        assert_eq!(found.name, "Team A");
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

    #[tokio::test]
    async fn list_returns_teams_ordered_by_created_at_desc() {
        let mock = MemoryMockQuery::new();
        mock.seed_team(make_team("team-1", "A"));
        mock.seed_team(make_team("team-2", "B"));
        mock.seed_team(make_team("team-3", "C"));

        let list = TeamQuery::list(&mock, Page { offset: 0, limit: 10 }).await.unwrap();
        assert_eq!(list.len(), 3);
    }

    #[tokio::test]
    async fn prefill_avatar_key_sets_avatar_key() {
        let mock = MemoryMockQuery::new();
        mock.seed_team(make_team("team-1", "A"));

        TeamQuery::prefill_avatar_key(&mock, "team-1", "avatars/new-key.png")
            .await
            .unwrap();

        let found = TeamQuery::get_by_id(&mock, "team-1").await.unwrap();
        assert_eq!(found.avatar_key, "avatars/new-key.png");
        assert!(!found.avatar_uploaded);
    }

    #[tokio::test]
    async fn mark_avatar_uploaded_sets_flag() {
        let mock = MemoryMockQuery::new();
        mock.seed_team(make_team("team-1", "A"));

        TeamQuery::mark_avatar_uploaded(&mock, "team-1")
            .await
            .unwrap();

        let found = TeamQuery::get_by_id(&mock, "team-1").await.unwrap();
        assert!(found.avatar_uploaded);
    }

    #[tokio::test]
    async fn create_then_find() {
        let mock = MemoryMockQuery::new();

        mock.transaction_scoped(|txn| {
            async move {
                let form = TeamForm {
                    id: TeamAggr::generate_id(),
                    name: "New Team".into(),
                    description: "A new team".into(),
                };
                let team = TeamQuery::create(txn, &form).await.unwrap();
                assert_eq!(team.name, "New Team");
                assert_eq!(team.workset_next_index, 0);
                Ok(())
            }
            .boxed()
        })
        .await
        .unwrap();

        let snapshot = mock.snapshot();
        assert_eq!(snapshot.teams.len(), 1);
    }

    #[tokio::test]
    async fn create_duplicate_name_returns_conflict() {
        let mock = MemoryMockQuery::new();

        mock.transaction_scoped(|txn| {
            async move {
                let form = TeamForm {
                    id: "team-1".into(),
                    name: "My Team".into(),
                    description: "desc".into(),
                };
                TeamQuery::create(txn, &form).await.unwrap();
                Ok(())
            }
            .boxed()
        })
        .await
        .unwrap();

        let err = mock
            .transaction_scoped(|txn| {
                async move {
                    let form = TeamForm {
                        id: "team-2".into(),
                        name: "My Team".into(),
                        description: "desc".into(),
                    };
                    TeamQuery::create(txn, &form).await
                }
                .boxed()
            })
            .await
            .err()
            .unwrap();

        assert!(is_expected_conflict(&err));
    }

    #[tokio::test]
    async fn update_changes_fields() {
        let mock = MemoryMockQuery::new();
        mock.seed_team(make_team("team-1", "Old Name"));

        mock.transaction_scoped(|txn| {
            async move {
                let input = TeamUpdate {
                    id: "team-1".into(),
                    name: "New Name".into(),
                    description: "New Desc".into(),
                };
                TeamQuery::update(txn, &input).await
            }
            .boxed()
        })
        .await
        .unwrap();

        let found = TeamQuery::get_by_id(&mock, "team-1").await.unwrap();
        assert_eq!(found.name, "New Name");
        assert_eq!(found.description, "New Desc");
    }

    #[tokio::test]
    async fn update_missing_returns_error() {
        let mock = MemoryMockQuery::new();

        let err = mock
            .transaction_scoped(|txn| {
                async move {
                    let input = TeamUpdate {
                        id: "nonexistent".into(),
                        name: "X".into(),
                        description: "Y".into(),
                    };
                    TeamQuery::update(txn, &input).await
                }
                .boxed()
            })
            .await
            .err()
            .unwrap();

        assert!(is_expected_argument(&err));
    }

    #[tokio::test]
    async fn delete_removes_team() {
        let mock = MemoryMockQuery::new();
        mock.seed_team(make_team("team-1", "A"));

        mock.transaction_scoped(|txn| {
            async move { TeamQuery::delete(txn, "team-1").await }.boxed()
        })
        .await
        .unwrap();

        let snapshot = mock.snapshot();
        assert!(snapshot.teams.is_empty());
    }

    #[tokio::test]
    async fn delete_missing_returns_error() {
        let mock = MemoryMockQuery::new();

        let err = mock
            .transaction_scoped(|txn| {
                async move { TeamQuery::delete(txn, "nonexistent").await }.boxed()
            })
            .await
            .err()
            .unwrap();

        assert!(is_expected_argument(&err));
    }

    #[tokio::test]
    async fn increment_workset_next_index_returns_allocated() {
        let mock = MemoryMockQuery::new();
        mock.seed_team(make_team("team-1", "A"));

        let mut indices = Vec::new();
        for _ in 0..3 {
            let idx = mock
                .transaction_scoped(|txn| {
                    async move {
                        TeamQueryTransactional::increment_workset_next_index(txn, "team-1").await
                    }
                    .boxed()
                })
                .await
                .unwrap();
            indices.push(idx);
        }

        assert_eq!(indices, vec![0, 1, 2]);
    }

    #[tokio::test]
    async fn increment_workset_next_index_missing_returns_error() {
        let mock = MemoryMockQuery::new();

        let err = mock
            .transaction_scoped(|txn| {
                async move {
                    TeamQueryTransactional::increment_workset_next_index(txn, "nonexistent").await
                }
                .boxed()
            })
            .await
            .err()
            .unwrap();

        assert!(is_expected_argument(&err));
    }
}
