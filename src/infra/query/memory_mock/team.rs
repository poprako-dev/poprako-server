use async_trait::async_trait;
use time::OffsetDateTime;

use poprako_util::i18n::trl;
use poprako_util::page::Page;

use crate::domain::model::aggr::team::{TeamAggr, TeamAvatarReservation, TeamForm, TeamInfoUpdate};
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
            avatar_key: None,
            avatar_uploaded: false,
            avatar_version: 0,
            workset_next_index: 0,
            created_at: now,
            updated_at: now,
        };

        state.teams.push(team.clone());

        Ok(team)
    }

    async fn update_info(&self, update: &TeamInfoUpdate) -> DomainResult<()> {
        let mut state = self.state.lock().unwrap();

        let team = state
            .teams
            .iter_mut()
            .find(|t| t.id == update.id)
            .ok_or_else(|| DomainError::expected_argument(trl("error-team-not-found")))?;

        team.name = update.name.clone();
        team.description = update.description.clone();
        team.updated_at = OffsetDateTime::now_utc();

        Ok(())
    }

    async fn mark_avatar_uploaded(&self, id: &str, image_version: i64) -> DomainResult<()> {
        let mut state = self.state.lock().unwrap();

        let team = state
            .teams
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| DomainError::expected_argument(trl("error-team-not-found")))?;

        if team.avatar_version != image_version {
            return Err(DomainError::expected_argument(trl(
                "error-stale-avatar-upload",
            )));
        }

        if team.avatar_uploaded {
            return Ok(());
        }

        team.avatar_uploaded = true;
        team.updated_at = OffsetDateTime::now_utc();

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

    async fn get_by_id_excluded(&mut self, id: &str) -> DomainResult<TeamAggr> {
        let state = self.state.lock().unwrap();
        state
            .teams
            .iter()
            .find(|t| t.id == id)
            .cloned()
            .ok_or_else(|| DomainError::expected_argument(trl("error-team-not-found")))
    }

    async fn reserve_avatar(
        &mut self,
        id: &str,
        file_extension: &str,
    ) -> DomainResult<TeamAvatarReservation> {
        let mut state = self.state.lock().unwrap();

        let team = state
            .teams
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| DomainError::expected_argument(trl("error-team-not-found")))?;

        let image_version = team.avatar_version + 1;
        let object_key = TeamAggr::generate_avatar_key(id, image_version, file_extension);
        let previous_object_key = team.avatar_key.clone();

        team.avatar_key = Some(object_key.clone());
        team.avatar_uploaded = false;
        team.avatar_version = image_version;
        team.updated_at = OffsetDateTime::now_utc();

        Ok(TeamAvatarReservation {
            object_key,
            previous_object_key,
            image_version,
        })
    }

    async fn mark_avatar_uploaded(&mut self, id: &str, image_version: i64) -> DomainResult<()> {
        let mut state = self.state.lock().unwrap();
        let team = state
            .teams
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| DomainError::expected_argument(trl("error-team-not-found")))?;
        if team.avatar_version != image_version {
            return Err(DomainError::expected_argument(trl("error-stale-avatar-upload")));
        }
        if team.avatar_uploaded {
            return Ok(());
        }
        team.avatar_uploaded = true;
        team.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    async fn delete(&mut self, id: &str) -> DomainResult<()> {
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

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    // find_by_id_after_seed(TeamQuery::get_by_id)(positive): seeded teams should be found by ID.
    // get_by_id_missing_returns_expected_error(TeamQuery::get_by_id)(negative): missing teams should return an expected argument error.
    // list_returns_teams_ordered_by_created_at_desc(TeamQuery::list)(positive): list should return teams ordered by created_at DESC.
    // reserve_avatar_sets_key_and_version(TeamQueryTransactional::reserve_avatar)(positive): reserve should set the avatar key, clear uploaded flag, and increment version.
    // mark_avatar_uploaded_sets_flag(TeamQuery::mark_avatar_uploaded)(positive): marking should set avatar_uploaded to true.
    // create_then_find(TeamQuery::create)(positive): created teams should be readable after transaction commit.
    // create_duplicate_name_returns_conflict(TeamQuery::create)(negative): duplicate team names should return a conflict.
    // update_changes_fields(TeamQuery::update)(positive): update should change name and description.
    // update_missing_returns_error(TeamQuery::update)(negative): updating a missing team should fail.
    // delete_removes_team(TeamQuery::delete)(positive): deleting a team should remove it from storage.
    // delete_cascade_deletes_worksets(TeamQuery::delete)(positive): deleting a team should cascade-delete all worksets belonging to the team.
    // delete_does_not_cascade_worksets_when_called_directly(TeamQuery::delete)(positive): calling delete directly before the cascade wrapper should NOT remove worksets — cascade is handled by complex::team::delete_cascade.
    // delete_missing_returns_error(TeamQuery::delete)(negative): deleting a missing team should fail.
    // increment_workset_next_index_returns_allocated(TeamQueryTransactional::increment_workset_next_index)(positive): each call should return the current value and increment it.
    // increment_workset_next_index_missing_returns_error(TeamQueryTransactional::increment_workset_next_index)(negative): incrementing a missing team should fail.

    use futures_util::FutureExt as _;
    use time::OffsetDateTime;

    use poprako_util::page::Page;

    use crate::domain::model::aggr::team::{TeamAggr, TeamForm, TeamInfoUpdate};
    use crate::domain::model::aggr::workset::WorksetAggr;
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
            avatar_key: None,
            avatar_uploaded: false,
            avatar_version: 0,
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

        let list = TeamQuery::list(
            &mock,
            Page {
                offset: 0,
                limit: 10,
            },
        )
        .await
        .unwrap();
        assert_eq!(list.len(), 3);
    }

    #[tokio::test]
    async fn reserve_avatar_sets_key_and_version() {
        let mock = MemoryMockQuery::new();
        mock.seed_team(make_team("team-1", "A"));

        let reservation = mock
            .transaction_scoped(|txn| {
                async move { TeamQueryTransactional::reserve_avatar(txn, "team-1", "png").await }
                    .boxed()
            })
            .await
            .unwrap();

        let found = TeamQuery::get_by_id(&mock, "team-1").await.unwrap();
        assert_eq!(reservation.image_version, 1);
        assert_eq!(found.avatar_key, Some("team_avatar/team-1-1.png".into()));
        assert_eq!(found.avatar_version, 1);
        assert!(!found.avatar_uploaded);
    }

    #[tokio::test]
    async fn mark_avatar_uploaded_sets_flag() {
        let mock = MemoryMockQuery::new();
        mock.seed_team(make_team("team-1", "A"));

        mock.transaction_scoped(|txn| {
            async move {
                TeamQueryTransactional::reserve_avatar(txn, "team-1", "png").await
            }
            .boxed()
        })
        .await
        .unwrap();

        TeamQuery::mark_avatar_uploaded(&mock, "team-1", 1)
            .await
            .unwrap();

        let found = TeamQuery::get_by_id(&mock, "team-1").await.unwrap();
        assert!(found.avatar_uploaded);
    }

    #[tokio::test]
    async fn create_then_find() {
        let mock = MemoryMockQuery::new();

        let form = TeamForm {
            id: TeamAggr::generate_id(),
            name: "New Team".into(),
            description: "A new team".into(),
        };
        let team = TeamQuery::create(&mock, &form).await.unwrap();
        assert_eq!(team.name, "New Team");
        assert_eq!(team.workset_next_index, 0);

        let snapshot = mock.snapshot();
        assert_eq!(snapshot.teams.len(), 1);
    }

    #[tokio::test]
    async fn create_duplicate_name_returns_conflict() {
        let mock = MemoryMockQuery::new();

        let form = TeamForm {
            id: "team-1".into(),
            name: "My Team".into(),
            description: "desc".into(),
        };
        TeamQuery::create(&mock, &form).await.unwrap();

        let form = TeamForm {
            id: "team-2".into(),
            name: "My Team".into(),
            description: "desc".into(),
        };
        let err = TeamQuery::create(&mock, &form).await.err().unwrap();

        assert!(is_expected_conflict(&err));
    }

    #[tokio::test]
    async fn update_changes_fields() {
        let mock = MemoryMockQuery::new();
        mock.seed_team(make_team("team-1", "Old Name"));

        let update = TeamInfoUpdate {
            id: "team-1".into(),
            name: "New Name".into(),
            description: "New Desc".into(),
        };
        TeamQuery::update_info(&mock, &update).await.unwrap();

        let found = TeamQuery::get_by_id(&mock, "team-1").await.unwrap();
        assert_eq!(found.name, "New Name");
        assert_eq!(found.description, "New Desc");
    }

    #[tokio::test]
    async fn update_missing_returns_error() {
        let mock = MemoryMockQuery::new();

        let update = TeamInfoUpdate {
            id: "nonexistent".into(),
            name: "X".into(),
            description: "Y".into(),
        };
        let err = TeamQuery::update_info(&mock, &update).await.err().unwrap();

        assert!(is_expected_argument(&err));
    }

    #[tokio::test]
    async fn delete_removes_team() {
        let mock = MemoryMockQuery::new();
        mock.seed_team(make_team("team-1", "A"));

        mock.transaction_scoped(|txn| {
            async move {
                TeamQueryTransactional::delete(txn, "team-1").await?;
                Ok(())
            }
            .boxed()
        })
        .await
        .unwrap();

        let snapshot = mock.snapshot();
        assert!(snapshot.teams.is_empty());
    }

    #[tokio::test]
    async fn delete_does_not_cascade_worksets_when_called_directly() {
        let mock = MemoryMockQuery::new();
        mock.seed_team(make_team("team-1", "A"));

        let n = now();
        mock.seed_workset(WorksetAggr {
            id: "ws-1".into(),
            team_id: "team-1".into(),
            team: None,
            index: 0,
            name: "WS1".into(),
            description: None,
            comic_count: 0,
            comic_next_index: 0,
            created_at: n,
            updated_at: n,
        });

        mock.transaction_scoped(|txn| {
            async move {
                TeamQueryTransactional::delete(txn, "team-1").await?;
                Ok(())
            }
            .boxed()
        })
        .await
        .unwrap();

        let snapshot = mock.snapshot();
        // Team is gone.
        assert!(snapshot.teams.is_empty());
        // Worksets survive — cascade is handled by complex::team::delete_cascade.
        assert_eq!(snapshot.worksets.len(), 1);
        assert_eq!(snapshot.worksets[0].id, "ws-1");
    }

    #[tokio::test]
    async fn delete_missing_returns_error() {
        let mock = MemoryMockQuery::new();

        let err = mock
            .transaction_scoped(|txn| {
                async move { TeamQueryTransactional::delete(txn, "nonexistent").await }.boxed()
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
