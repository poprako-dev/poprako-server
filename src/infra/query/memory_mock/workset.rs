use async_trait::async_trait;
use time::OffsetDateTime;

use poprako_util::i18n::trl;
use poprako_util::page::Page;

use crate::domain::model::aggr::workset::{WorksetAggr, WorksetForm, WorksetUpdate};
use crate::domain::query::workset::{WorksetQuery, WorksetQueryTransactional};
use crate::domain::result::{DomainError, DomainResult};
use crate::infra::query::memory_mock::{MemoryMockQuery, MemoryMockQueryTransactional};

// ── Query impls ────────────────────────────────────────────────────────────

#[async_trait]
impl WorksetQuery for MemoryMockQuery {
    async fn get_by_id(&self, id: &str) -> DomainResult<WorksetAggr> {
        let state = self.state.lock().unwrap();
        state
            .worksets
            .iter()
            .find(|w| w.id == id)
            .cloned()
            .ok_or_else(|| DomainError::expected_argument(trl("error-workset-not-found")))
    }

    async fn list(&self, team_id_filter: &str, page: Page) -> DomainResult<Vec<WorksetAggr>> {
        let state = self.state.lock().unwrap();
        let mut worksets: Vec<WorksetAggr> = state
            .worksets
            .iter()
            .filter(|w| w.team_id == team_id_filter)
            .cloned()
            .collect();

        // Preload the owning team on each workset.
        for workset in worksets.iter_mut() {
            if let Some(team) = state.teams.iter().find(|t| t.id == workset.team_id) {
                workset.team = Some(team.clone());
            }
        }

        // Sort by index ascending.
        worksets.sort_by_key(|w| w.index);

        let skip = page.offset;
        let take = page.limit;

        if skip >= worksets.len() {
            return Ok(Vec::new());
        }

        let end = std::cmp::min(skip + take, worksets.len());
        Ok(worksets[skip..end].to_vec())
    }

    async fn count(&self, team_id_filter: &str) -> DomainResult<i64> {
        let state = self.state.lock().unwrap();
        let total = state
            .worksets
            .iter()
            .filter(|w| w.team_id == team_id_filter)
            .count() as i64;
        Ok(total)
    }

    async fn update(&self, input: &WorksetUpdate) -> DomainResult<()> {
        let mut state = self.state.lock().unwrap();

        let workset = state
            .worksets
            .iter_mut()
            .find(|w| w.id == input.id)
            .ok_or_else(|| DomainError::expected_argument(trl("error-workset-not-found")))?;

        workset.name = input.name.clone();
        workset.description = input.description.clone();
        workset.updated_at = OffsetDateTime::now_utc();

        Ok(())
    }

    async fn delete(&self, id: &str) -> DomainResult<()> {
        let mut state = self.state.lock().unwrap();

        let pos = state
            .worksets
            .iter()
            .position(|w| w.id == id)
            .ok_or_else(|| DomainError::expected_argument(trl("error-workset-not-found")))?;

        state.worksets.remove(pos);

        Ok(())
    }
}

// ── QueryTransactional impls ───────────────────────────────────────────────

#[async_trait]
impl WorksetQueryTransactional for MemoryMockQueryTransactional {
    async fn create(&mut self, form: &WorksetForm) -> DomainResult<WorksetAggr> {
        let mut state = self.state.lock().unwrap();

        // Check uniqueness constraints.
        if state.worksets.iter().any(|w| w.id == form.id) {
            return Err(DomainError::expected_conflict(trl("error-already-exists")));
        }
        if state
            .worksets
            .iter()
            .any(|w| w.team_id == form.team_id && w.index == form.index)
        {
            return Err(DomainError::expected_conflict(trl("error-already-exists")));
        }

        let now = OffsetDateTime::now_utc();
        let workset = WorksetAggr {
            id: form.id.clone(),
            team_id: form.team_id.clone(),
            team: None,
            index: form.index,
            name: form.name.clone(),
            description: form.description.clone(),
            comic_count: 0,
            comic_next_index: 0,
            created_at: now,
            updated_at: now,
        };

        state.worksets.push(workset.clone());

        Ok(workset)
    }

    async fn update_comic_count(&mut self, id: &str, delta: i32) -> DomainResult<()> {
        let mut state = self.state.lock().unwrap();

        let workset = state
            .worksets
            .iter_mut()
            .find(|w| w.id == id)
            .ok_or_else(|| DomainError::expected_argument(trl("error-workset-not-found")))?;

        let new_count = std::cmp::max(0, workset.comic_count + delta);
        workset.comic_count = new_count;
        workset.updated_at = OffsetDateTime::now_utc();

        Ok(())
    }

    async fn increment_comic_next_index(&mut self, id: &str) -> DomainResult<i32> {
        let mut state = self.state.lock().unwrap();

        let workset = state
            .worksets
            .iter_mut()
            .find(|w| w.id == id)
            .ok_or_else(|| DomainError::expected_argument(trl("error-workset-not-found")))?;

        let allocated = workset.comic_next_index;
        workset.comic_next_index += 1;
        workset.updated_at = OffsetDateTime::now_utc();

        Ok(allocated)
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

// ── WorksetQuery forwarding on transactional handle ────────────────────────

#[async_trait]
impl WorksetQuery for MemoryMockQueryTransactional {
    async fn get_by_id(&self, id: &str) -> DomainResult<WorksetAggr> {
        let state = self.state.lock().unwrap();
        state
            .worksets
            .iter()
            .find(|w| w.id == id)
            .cloned()
            .ok_or_else(|| DomainError::expected_argument(trl("error-workset-not-found")))
    }

    async fn list(&self, team_id: &str, page: Page) -> DomainResult<Vec<WorksetAggr>> {
        let state = self.state.lock().unwrap();
        let filtered: Vec<WorksetAggr> = state
            .worksets
            .iter()
            .filter(|w| w.team_id == team_id)
            .cloned()
            .collect();
        let skip = page.offset;
        let take = page.limit;
        if skip >= filtered.len() {
            return Ok(Vec::new());
        }
        let end = std::cmp::min(skip + take, filtered.len());
        Ok(filtered[skip..end].to_vec())
    }

    async fn count(&self, team_id: &str) -> DomainResult<i64> {
        let state = self.state.lock().unwrap();
        Ok(state
            .worksets
            .iter()
            .filter(|w| w.team_id == team_id)
            .count() as i64)
    }

    async fn update(&self, input: &WorksetUpdate) -> DomainResult<()> {
        let mut state = self.state.lock().unwrap();
        let workset = state
            .worksets
            .iter_mut()
            .find(|w| w.id == input.id)
            .ok_or_else(|| DomainError::expected_argument(trl("error-workset-not-found")))?;
        workset.name = input.name.clone();
        workset.description = input.description.clone();
        workset.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    async fn delete(&self, id: &str) -> DomainResult<()> {
        let mut state = self.state.lock().unwrap();
        let pos = state
            .worksets
            .iter()
            .position(|w| w.id == id)
            .ok_or_else(|| DomainError::expected_argument(trl("error-workset-not-found")))?;
        state.worksets.remove(pos);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // find_by_id_after_seed(WorksetQuery::get_by_id)(positive): seeded worksets should be found by ID.
    // get_by_id_missing_returns_expected_error(WorksetQuery::get_by_id)(negative): missing worksets should return an expected argument error.
    // list_filters_by_team_and_paginates(WorksetQuery::list)(positive): list should filter by team and paginate correctly.
    // count_returns_correct_value(WorksetQuery::count)(positive): count should return the correct number of worksets for a team.
    // create_then_find(WorksetQueryTransactional::create)(positive): created worksets should be readable after transaction commit.
    // create_duplicate_index_returns_conflict(WorksetQueryTransactional::create)(negative): duplicate team+index should return an expected conflict.
    // update_changes_fields(WorksetQuery::update)(positive): update should change name and description.
    // update_missing_returns_error(WorksetQuery::update)(negative): updating a missing workset should fail.
    // update_comic_count_applies_delta(WorksetQueryTransactional::update_comic_count)(positive): delta should be applied and clamped to zero.
    // update_comic_count_missing_returns_error(WorksetQueryTransactional::update_comic_count)(negative): updating a missing workset should fail.
    // increment_comic_next_index_returns_allocated(WorksetQueryTransactional::increment_comic_next_index)(positive): each call should return the current value and increment it.
    // increment_comic_next_index_missing_returns_error(WorksetQueryTransactional::increment_comic_next_index)(negative): incrementing a missing workset should fail.
    // delete_removes_workset(WorksetQuery::delete)(positive): deleting a workset should remove it from storage.
    // delete_missing_returns_error(WorksetQuery::delete)(negative): deleting a missing workset should fail.

    use futures_util::FutureExt as _;
    use time::OffsetDateTime;

    use poprako_util::page::Page;

    use crate::domain::model::aggr::workset::{WorksetAggr, WorksetForm, WorksetUpdate};
    use crate::domain::query::Transactional;
    use crate::domain::query::workset::{WorksetQuery, WorksetQueryTransactional};
    use crate::infra::query::memory_mock::MemoryMockQuery;
    use crate::test_util::is_expected_argument;
    use crate::test_util::is_expected_conflict;

    fn now() -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }

    fn make_workset(id: &str, team_id: &str, index: i32, name: &str) -> WorksetAggr {
        let n = now();
        WorksetAggr {
            id: id.into(),
            team_id: team_id.into(),
            team: None,
            index,
            name: name.into(),
            description: None,
            comic_count: 0,
            comic_next_index: 0,
            created_at: n,
            updated_at: n,
        }
    }

    #[tokio::test]
    async fn find_by_id_after_seed() {
        let mock = MemoryMockQuery::new();
        mock.seed_workset(make_workset("workset-1", "team-1", 0, "My Workset"));

        let found = WorksetQuery::get_by_id(&mock, "workset-1").await.unwrap();
        assert_eq!(found.id, "workset-1");
        assert_eq!(found.name, "My Workset");
    }

    #[tokio::test]
    async fn get_by_id_missing_returns_expected_error() {
        let mock = MemoryMockQuery::new();

        let err = WorksetQuery::get_by_id(&mock, "nonexistent")
            .await
            .err()
            .unwrap();
        assert!(is_expected_argument(&err));
    }

    #[tokio::test]
    async fn list_filters_by_team_and_paginates() {
        let mock = MemoryMockQuery::new();
        mock.seed_workset(make_workset("workset-1", "team-1", 2, "B"));
        mock.seed_workset(make_workset("workset-2", "team-1", 1, "A"));
        mock.seed_workset(make_workset("workset-3", "team-2", 0, "Other"));

        let list = WorksetQuery::list(
            &mock,
            "team-1",
            Page {
                offset: 0,
                limit: 10,
            },
        )
        .await
        .unwrap();
        assert_eq!(list.len(), 2);
        // Should be ordered by index ascending.
        assert_eq!(list[0].id, "workset-2");
        assert_eq!(list[1].id, "workset-1");

        // Pagination: page.offset 1, page.limit 1
        let page = WorksetQuery::list(
            &mock,
            "team-1",
            Page {
                offset: 1,
                limit: 1,
            },
        )
        .await
        .unwrap();
        assert_eq!(page.len(), 1);
        assert_eq!(page[0].id, "workset-1");
    }

    #[tokio::test]
    async fn count_returns_correct_value() {
        let mock = MemoryMockQuery::new();
        mock.seed_workset(make_workset("workset-1", "team-1", 0, "A"));
        mock.seed_workset(make_workset("workset-2", "team-1", 1, "B"));
        mock.seed_workset(make_workset("workset-3", "team-2", 0, "C"));

        let total = WorksetQuery::count(&mock, "team-1").await.unwrap();
        assert_eq!(total, 2);

        let cnt_empty = WorksetQuery::count(&mock, "team-none").await.unwrap();
        assert_eq!(cnt_empty, 0);
    }

    #[tokio::test]
    async fn create_then_find() {
        let mock = MemoryMockQuery::new();

        mock.transaction_scoped(|txn| {
            async move {
                let form = WorksetForm {
                    id: WorksetAggr::generate_id(),
                    team_id: "team-1".into(),
                    index: 0,
                    name: "New Workset".into(),
                    description: Some("A description".into()),
                };
                let created = WorksetQueryTransactional::create(txn, &form).await.unwrap();
                assert_eq!(created.name, "New Workset");
                assert_eq!(created.comic_count, 0);
                Ok(())
            }
            .boxed()
        })
        .await
        .unwrap();

        let snapshot = mock.snapshot();
        assert_eq!(snapshot.worksets.len(), 1);
        assert_eq!(
            snapshot.worksets[0].description,
            Some("A description".into())
        );
    }

    #[tokio::test]
    async fn create_duplicate_index_returns_conflict() {
        let mock = MemoryMockQuery::new();

        mock.transaction_scoped(|txn| {
            async move {
                let form = WorksetForm {
                    id: "workset-1".into(),
                    team_id: "team-1".into(),
                    index: 0,
                    name: "First".into(),
                    description: None,
                };
                WorksetQueryTransactional::create(txn, &form).await.unwrap();
                Ok(())
            }
            .boxed()
        })
        .await
        .unwrap();

        let err = mock
            .transaction_scoped(|txn| {
                async move {
                    let form = WorksetForm {
                        id: "workset-2".into(),
                        team_id: "team-1".into(),
                        index: 0,
                        name: "Second".into(),
                        description: None,
                    };
                    WorksetQueryTransactional::create(txn, &form).await
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
        mock.seed_workset(make_workset("workset-1", "team-1", 0, "Old Name"));

        mock.transaction_scoped(|txn| {
            async move {
                let input = WorksetUpdate {
                    id: "workset-1".into(),
                    name: "New Name".into(),
                    description: Some("New Desc".into()),
                };
                WorksetQuery::update(txn, &input).await
            }
            .boxed()
        })
        .await
        .unwrap();

        let found = WorksetQuery::get_by_id(&mock, "workset-1").await.unwrap();
        assert_eq!(found.name, "New Name");
        assert_eq!(found.description, Some("New Desc".into()));
    }

    #[tokio::test]
    async fn update_missing_returns_error() {
        let mock = MemoryMockQuery::new();

        let err = mock
            .transaction_scoped(|txn| {
                async move {
                    let input = WorksetUpdate {
                        id: "nonexistent".into(),
                        name: "X".into(),
                        description: None,
                    };
                    WorksetQuery::update(txn, &input).await
                }
                .boxed()
            })
            .await
            .err()
            .unwrap();

        assert!(is_expected_argument(&err));
    }

    #[tokio::test]
    async fn update_comic_count_applies_delta() {
        let mock = MemoryMockQuery::new();
        mock.seed_workset(make_workset("workset-1", "team-1", 0, "Test"));

        // First, set comic_count to 5 manually.
        {
            let mut state = mock.state.lock().unwrap();
            state.worksets[0].comic_count = 5;
        }

        mock.transaction_scoped(|txn| {
            async move {
                WorksetQueryTransactional::update_comic_count(txn, "workset-1", 3)
                    .await
                    .unwrap();
                Ok(())
            }
            .boxed()
        })
        .await
        .unwrap();

        let found = WorksetQuery::get_by_id(&mock, "workset-1").await.unwrap();
        assert_eq!(found.comic_count, 8);

        // Clamp to zero.
        mock.transaction_scoped(|txn| {
            async move {
                WorksetQueryTransactional::update_comic_count(txn, "workset-1", -20)
                    .await
                    .unwrap();
                Ok(())
            }
            .boxed()
        })
        .await
        .unwrap();

        let found = WorksetQuery::get_by_id(&mock, "workset-1").await.unwrap();
        assert_eq!(found.comic_count, 0);
    }

    #[tokio::test]
    async fn update_comic_count_missing_returns_error() {
        let mock = MemoryMockQuery::new();

        let err = mock
            .transaction_scoped(|txn| {
                async move {
                    WorksetQueryTransactional::update_comic_count(txn, "nonexistent", 1)
                        .await
                }
                .boxed()
            })
            .await
            .err()
            .unwrap();

        assert!(is_expected_argument(&err));
    }

    #[tokio::test]
    async fn increment_comic_next_index_returns_allocated() {
        let mock = MemoryMockQuery::new();
        mock.seed_workset(make_workset("workset-1", "team-1", 0, "Test"));

        let mut indices = Vec::new();
        for _ in 0..3 {
            let idx = mock
                .transaction_scoped(|txn| {
                    async move {
                        WorksetQueryTransactional::increment_comic_next_index(txn, "workset-1")
                            .await
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
    async fn increment_comic_next_index_missing_returns_error() {
        let mock = MemoryMockQuery::new();

        let err = mock
            .transaction_scoped(|txn| {
                async move {
                    WorksetQueryTransactional::increment_comic_next_index(txn, "nonexistent").await
                }
                .boxed()
            })
            .await
            .err()
            .unwrap();

        assert!(is_expected_argument(&err));
    }

    #[tokio::test]
    async fn delete_removes_workset() {
        let mock = MemoryMockQuery::new();
        mock.seed_workset(make_workset("workset-1", "team-1", 0, "Test"));

        mock.transaction_scoped(|txn| {
            async move { WorksetQuery::delete(txn, "workset-1").await }.boxed()
        })
        .await
        .unwrap();

        let snapshot = mock.snapshot();
        assert!(snapshot.worksets.is_empty());
    }

    #[tokio::test]
    async fn delete_missing_returns_error() {
        let mock = MemoryMockQuery::new();

        let err = mock
            .transaction_scoped(|txn| {
                async move { WorksetQuery::delete(txn, "nonexistent").await }.boxed()
            })
            .await
            .err()
            .unwrap();

        assert!(is_expected_argument(&err));
    }
}
