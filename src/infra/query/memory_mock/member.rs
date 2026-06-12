use async_trait::async_trait;
use time::OffsetDateTime;

use poprako_util::i18n::trl;
use poprako_util::page::Page;

use crate::domain::model::aggr::member::{MemberAggr, MemberForm, MemberRoleUpdate};
use crate::domain::model::value::member_inclusion::MemberInclusion;
use crate::domain::model::value::role::RoleFlag;
use crate::domain::query::member::{MemberQuery, MemberQueryTransactional};
use crate::domain::result::{DomainError, DomainResult};
use crate::infra::query::memory_mock::{MemoryMockQuery, MemoryMockQueryTransactional};

// ── Query impls ────────────────────────────────────────────────────────────

#[async_trait]
impl MemberQuery for MemoryMockQuery {
    async fn get_by_id(&self, id: &str) -> DomainResult<MemberAggr> {
        let state = self.state.lock().unwrap();
        state
            .members
            .iter()
            .find(|m| m.id == id)
            .cloned()
            .ok_or_else(|| DomainError::expected_argument(trl("error-member-not-found")))
    }

    async fn get_by_user_and_team_id(
        &self,
        user_id: &str,
        team_id: &str,
    ) -> DomainResult<MemberAggr> {
        let state = self.state.lock().unwrap();
        state
            .members
            .iter()
            .find(|m| m.user_id == user_id && m.team_id == team_id)
            .cloned()
            .ok_or_else(|| DomainError::expected_argument(trl("error-member-not-found")))
    }

    async fn list_by_team_id(
        &self,
        team_id: &str,
        keyword: Option<&str>,
        role: Option<RoleFlag>,
        page: Page,
        includes: &MemberInclusion,
    ) -> DomainResult<Vec<MemberAggr>> {
        let state = self.state.lock().unwrap();

        let mut filtered: Vec<MemberAggr> = state
            .members
            .iter()
            .filter(|member| member.team_id == team_id)
            .filter(|member| {
                keyword.is_none_or(|text| {
                    member
                        .user_nickname
                        .to_lowercase()
                        .contains(&text.to_lowercase())
                })
            })
            .filter(|member| role.is_none_or(|flag| member.has_any_role(&[flag])))
            .cloned()
            .collect();

        if includes.user {
            for member in filtered.iter_mut() {
                member.user = state
                    .users
                    .iter()
                    .find(|user| user.id == member.user_id)
                    .cloned();
            }
        }

        if includes.team {
            for member in filtered.iter_mut() {
                member.team = state
                    .teams
                    .iter()
                    .find(|team| team.id == member.team_id)
                    .cloned();
            }
        }

        let skip = page.offset;
        let take = page.limit;

        if skip >= filtered.len() {
            return Ok(Vec::new());
        }

        let end = std::cmp::min(skip + take, filtered.len());
        Ok(filtered[skip..end].to_vec())
    }

    async fn list_by_user_id(
        &self,
        user_id: &str,
        page: Page,
        includes: &MemberInclusion,
    ) -> DomainResult<Vec<MemberAggr>> {
        let state = self.state.lock().unwrap();

        let mut filtered: Vec<MemberAggr> = state
            .members
            .iter()
            .filter(|member| member.user_id == user_id)
            .cloned()
            .collect();

        if includes.user {
            for member in filtered.iter_mut() {
                member.user = state
                    .users
                    .iter()
                    .find(|user| user.id == member.user_id)
                    .cloned();
            }
        }

        if includes.team {
            for member in filtered.iter_mut() {
                member.team = state
                    .teams
                    .iter()
                    .find(|team| team.id == member.team_id)
                    .cloned();
            }
        }

        let skip = page.offset;
        let take = page.limit;

        if skip >= filtered.len() {
            return Ok(Vec::new());
        }

        let end = std::cmp::min(skip + take, filtered.len());
        Ok(filtered[skip..end].to_vec())
    }

    async fn exist_by_user_and_team_id(&self, user_id: &str, team_id: &str) -> DomainResult<bool> {
        let state = self.state.lock().unwrap();
        Ok(state
            .members
            .iter()
            .any(|m| m.user_id == user_id && m.team_id == team_id))
    }
}

// ── QueryTransactional impls ───────────────────────────────────────────────

#[async_trait]
impl MemberQueryTransactional for MemoryMockQueryTransactional {
    async fn create(&mut self, form: &MemberForm) -> DomainResult<MemberAggr> {
        let mut state = self.state.lock().unwrap();

        // Check uniqueness constraints.
        if state.members.iter().any(|m| m.id == form.id) {
            return Err(DomainError::expected_conflict(trl("error-already-exists")));
        }
        if state
            .members
            .iter()
            .any(|m| m.user_id == form.user_id && m.team_id == form.team_id)
        {
            return Err(DomainError::expected_conflict(trl("error-already-exists")));
        }

        let now = OffsetDateTime::now_utc();
        let roles = form.role_mask;

        let member = MemberAggr {
            id: form.id.clone(),
            user_id: form.user_id.clone(),
            user_nickname: form.user_nickname.clone(),
            user: None,
            team_id: form.team_id.clone(),
            team: None,
            assigned_raw_provider_at: roles.has_role(RoleFlag::RawProvider).then_some(now),
            assigned_translator_at: roles.has_role(RoleFlag::Translator).then_some(now),
            assigned_proofreader_at: roles.has_role(RoleFlag::Proofreader).then_some(now),
            assigned_typesetter_at: roles.has_role(RoleFlag::Typesetter).then_some(now),
            assigned_redrawer_at: roles.has_role(RoleFlag::Redrawer).then_some(now),
            assigned_reviewer_at: roles.has_role(RoleFlag::Reviewer).then_some(now),
            assigned_publisher_at: roles.has_role(RoleFlag::Publisher).then_some(now),
            assigned_admin_at: roles.has_role(RoleFlag::Admin).then_some(now),
            assigned_assistant_at: roles.has_role(RoleFlag::Assistant).then_some(now),
            user_last_active_at: now,
            created_at: now,
            updated_at: now,
        };

        state.members.push(member.clone());

        Ok(member)
    }

    async fn get_by_id_excluded(&mut self, id: &str) -> DomainResult<MemberAggr> {
        let state = self.state.lock().unwrap();
        state
            .members
            .iter()
            .find(|m| m.id == id)
            .cloned()
            .ok_or_else(|| DomainError::expected_argument(trl("error-member-not-found")))
    }

    async fn get_by_user_and_team_id_excluded(
        &mut self,
        user_id: &str,
        team_id: &str,
    ) -> DomainResult<MemberAggr> {
        let state = self.state.lock().unwrap();
        state
            .members
            .iter()
            .find(|m| m.user_id == user_id && m.team_id == team_id)
            .cloned()
            .ok_or_else(|| DomainError::expected_argument(trl("error-member-not-found")))
    }

    async fn update_user_nickname(&mut self, user_id: &str, nickname: &str) -> DomainResult<()> {
        let mut state = self.state.lock().unwrap();
        let now = OffsetDateTime::now_utc();

        for member in state.members.iter_mut() {
            if member.user_id == user_id {
                member.user_nickname = nickname.to_string();
                member.updated_at = now;
            }
        }

        Ok(())
    }

    async fn touch_last_active(&mut self, user_id: &str) -> DomainResult<()> {
        let mut state = self.state.lock().unwrap();
        let now = OffsetDateTime::now_utc();

        for member in state.members.iter_mut() {
            if member.user_id == user_id {
                member.user_last_active_at = now;
                member.updated_at = now;
            }
        }

        Ok(())
    }

    async fn list_by_user_id_excluded(&mut self, user_id: &str) -> DomainResult<Vec<MemberAggr>> {
        let state = self.state.lock().unwrap();
        let members: Vec<MemberAggr> = state
            .members
            .iter()
            .filter(|m| m.user_id == user_id)
            .cloned()
            .collect();
        Ok(members)
    }

    async fn update_roles(&mut self, update_data: &MemberRoleUpdate) -> DomainResult<()> {
        let mut state = self.state.lock().unwrap();

        let member = state
            .members
            .iter_mut()
            .find(|m| m.id == update_data.id)
            .ok_or_else(|| DomainError::expected_argument(trl("error-member-not-found")))?;

        let now = OffsetDateTime::now_utc();
        let roles = update_data.role_mask;

        member.assigned_raw_provider_at = roles.has_role(RoleFlag::RawProvider).then_some(now);
        member.assigned_translator_at = roles.has_role(RoleFlag::Translator).then_some(now);
        member.assigned_proofreader_at = roles.has_role(RoleFlag::Proofreader).then_some(now);
        member.assigned_typesetter_at = roles.has_role(RoleFlag::Typesetter).then_some(now);
        member.assigned_redrawer_at = roles.has_role(RoleFlag::Redrawer).then_some(now);
        member.assigned_reviewer_at = roles.has_role(RoleFlag::Reviewer).then_some(now);
        member.assigned_publisher_at = roles.has_role(RoleFlag::Publisher).then_some(now);
        member.assigned_admin_at = roles.has_role(RoleFlag::Admin).then_some(now);
        member.assigned_assistant_at = roles.has_role(RoleFlag::Assistant).then_some(now);
        member.updated_at = now;

        Ok(())
    }

    async fn delete(&mut self, id: &str) -> DomainResult<()> {
        let mut state = self.state.lock().unwrap();
        let pos = state
            .members
            .iter()
            .position(|m| m.id == id)
            .ok_or_else(|| DomainError::expected_argument(trl("error-member-not-found")))?;
        state.members.remove(pos);
        Ok(())
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    // duplicate_user_team_returns_conflict(MemberQueryTransactional::create)(negative): duplicate user-team membership should return an expected conflict.
    // update_user_nickname_updates_all_members_for_a_user(update_user_nickname)(positive): update_user_nickname should update the nickname on all members belonging to the user.
    // update_user_nickname_succeeds_when_user_has_no_members(update_user_nickname)(positive): update_user_nickname should succeed (no-op) when the user has no members.
    // get_by_id_after_seed(MemberQuery::get_by_id)(positive): seeded members should be found by ID.
    // get_by_id_missing_returns_expected_error(MemberQuery::get_by_id)(negative): missing members should return an expected argument error.
    // get_by_user_and_team_id_after_seed(MemberQuery::get_by_user_and_team_id)(positive): seeded members should be found by user+team.
    // get_by_user_and_team_id_missing_returns_expected_error(MemberQuery::get_by_user_and_team_id)(negative): missing members should return an expected argument error.
    // list_filters_by_team_keyword_role(MemberQuery::list)(positive): list should filter by team, keyword, and role.
    // exist_by_user_and_team_id_true(MemberQuery::exist_by_user_and_team_id)(positive): exist check should return true for existing member.
    // exist_by_user_and_team_id_false(MemberQuery::exist_by_user_and_team_id)(positive): exist check should return false for non-existing member.
    // list_user_filter_returns_user_members(MemberQuery::list)(positive): user filter should return all memberships for a user.
    // update_roles_replaces_all_roles(MemberQueryTransactional::update_roles)(positive): update_roles should clear all roles and set only those in the mask.
    // update_roles_missing_returns_error(MemberQueryTransactional::update_roles)(negative): updating roles on a missing member should fail.
    // delete_removes_member(MemberQueryTransactional::delete)(positive): deleting a member should remove it from storage.
    // delete_missing_returns_error(MemberQueryTransactional::delete)(negative): deleting a missing member should fail.

    use futures_util::FutureExt as _;

    use time::OffsetDateTime;

    use poprako_util::page::Page;

    use crate::domain::model::aggr::member::{MemberAggr, MemberForm, MemberRoleUpdate};
    use crate::domain::model::value::member_inclusion::MemberInclusion;
    use crate::domain::model::value::role::{RoleFlag, RoleMask};
    use crate::domain::query::Transactional;
    use crate::domain::query::member::{MemberQuery, MemberQueryTransactional};
    use crate::infra::query::memory_mock::MemoryMockQuery;
    use crate::test_util::{is_expected_argument, is_expected_conflict};

    fn now() -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }

    fn make_member(id: &str, user_id: &str, team_id: &str, roles: RoleMask) -> MemberAggr {
        let n = now();
        MemberAggr {
            id: id.into(),
            user_id: user_id.into(),
            user_nickname: "nick".into(),
            user: None,
            team_id: team_id.into(),
            team: None,
            assigned_raw_provider_at: roles.has_role(RoleFlag::RawProvider).then_some(n),
            assigned_translator_at: roles.has_role(RoleFlag::Translator).then_some(n),
            assigned_proofreader_at: roles.has_role(RoleFlag::Proofreader).then_some(n),
            assigned_typesetter_at: roles.has_role(RoleFlag::Typesetter).then_some(n),
            assigned_redrawer_at: roles.has_role(RoleFlag::Redrawer).then_some(n),
            assigned_reviewer_at: roles.has_role(RoleFlag::Reviewer).then_some(n),
            assigned_publisher_at: roles.has_role(RoleFlag::Publisher).then_some(n),
            assigned_admin_at: roles.has_role(RoleFlag::Admin).then_some(n),
            assigned_assistant_at: roles.has_role(RoleFlag::Assistant).then_some(n),
            user_last_active_at: n,
            created_at: n,
            updated_at: n,
        }
    }

    #[tokio::test]
    async fn duplicate_user_team_returns_conflict() {
        let mock = MemoryMockQuery::new();

        mock.transaction_scoped(|txn| {
            async move {
                let form = MemberForm {
                    id: MemberAggr::generate_id(),
                    user_id: "user-1".into(),
                    user_nickname: "nick".into(),
                    team_id: "team-1".into(),
                    role_mask: RoleMask::from(RoleFlag::Admin),
                };
                MemberQueryTransactional::create(txn, &form).await.unwrap();
                Ok(())
            }
            .boxed()
        })
        .await
        .unwrap();

        let err = mock
            .transaction_scoped(|txn| {
                async move {
                    let form = MemberForm {
                        id: MemberAggr::generate_id(),
                        user_id: "user-1".into(),
                        user_nickname: "nick".into(),
                        team_id: "team-1".into(),
                        role_mask: RoleMask::from(RoleFlag::Translator),
                    };
                    MemberQueryTransactional::create(txn, &form).await
                }
                .boxed()
            })
            .await
            .err()
            .unwrap();

        assert!(is_expected_conflict(&err));
    }

    #[tokio::test]
    async fn update_user_nickname_updates_all_members_for_a_user() {
        let mock = MemoryMockQuery::new();

        // Create two members for the same user in different teams.
        mock.transaction_scoped(|txn| {
            async move {
                let form1 = MemberForm {
                    id: MemberAggr::generate_id(),
                    user_id: "user-1".into(),
                    user_nickname: "OldNick".into(),
                    team_id: "team-1".into(),
                    role_mask: RoleMask::from(RoleFlag::Admin),
                };
                MemberQueryTransactional::create(txn, &form1).await.unwrap();

                let form2 = MemberForm {
                    id: MemberAggr::generate_id(),
                    user_id: "user-1".into(),
                    user_nickname: "OldNick".into(),
                    team_id: "team-2".into(),
                    role_mask: RoleMask::from(RoleFlag::Translator),
                };
                MemberQueryTransactional::create(txn, &form2).await.unwrap();

                Ok(())
            }
            .boxed()
        })
        .await
        .unwrap();

        // Update the nickname.
        mock.transaction_scoped(|txn| {
            async move {
                MemberQueryTransactional::update_user_nickname(
                    txn,
                    "user-1",
                    "NewNick",
                )
                .await
            }
            .boxed()
        })
        .await
        .unwrap();

        let snapshot = mock.snapshot();
        assert_eq!(snapshot.members.len(), 2);

        for member in &snapshot.members {
            assert_eq!(member.user_id, "user-1");
            assert_eq!(member.user_nickname, "NewNick");
        }
    }

    #[tokio::test]
    async fn update_user_nickname_succeeds_when_user_has_no_members() {
        let mock = MemoryMockQuery::new();

        let result = mock
            .transaction_scoped(|txn| {
                async move {
                    MemberQueryTransactional::update_user_nickname(txn, "no-such-user", "Nick")
                        .await
                }
                .boxed()
            })
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn get_by_id_after_seed() {
        let mock = MemoryMockQuery::new();
        mock.seed_member(make_member(
            "member-1",
            "user-1",
            "team-1",
            RoleFlag::Admin.into(),
        ));

        let found = MemberQuery::get_by_id(&mock, "member-1").await.unwrap();
        assert_eq!(found.id, "member-1");
        assert!(found.has_any_role(&[RoleFlag::Admin]));
    }

    #[tokio::test]
    async fn get_by_id_missing_returns_expected_error() {
        let mock = MemoryMockQuery::new();

        let err = MemberQuery::get_by_id(&mock, "nonexistent")
            .await
            .err()
            .unwrap();
        assert!(is_expected_argument(&err));
    }

    #[tokio::test]
    async fn get_by_user_and_team_id_after_seed() {
        let mock = MemoryMockQuery::new();
        mock.seed_member(make_member(
            "member-1",
            "user-1",
            "team-1",
            RoleFlag::Admin.into(),
        ));

        let found = MemberQuery::get_by_user_and_team_id(&mock, "user-1", "team-1")
            .await
            .unwrap();
        assert_eq!(found.id, "member-1");
    }

    #[tokio::test]
    async fn get_by_user_and_team_id_missing_returns_expected_error() {
        let mock = MemoryMockQuery::new();

        let err = MemberQuery::get_by_user_and_team_id(&mock, "user-1", "team-1")
            .await
            .err()
            .unwrap();
        assert!(is_expected_argument(&err));
    }

    #[tokio::test]
    async fn list_filters_by_team_keyword_role() {
        let mock = MemoryMockQuery::new();

        mock.seed_member({
            let mut m = make_member("m-1", "u-1", "team-1", RoleFlag::Admin.into());
            m.user_nickname = "Alice".into();
            m
        });
        mock.seed_member({
            let mut m = make_member("m-2", "u-2", "team-1", RoleFlag::Translator.into());
            m.user_nickname = "Bob".into();
            m
        });
        mock.seed_member({
            let mut m = make_member("m-3", "u-3", "team-2", RoleFlag::Admin.into());
            m.user_nickname = "Charlie".into();
            m
        });

        // List by team only.
        let list = MemberQuery::list_by_team_id(
            &mock,
            "team-1",
            None,
            None,
            Page {
                offset: 0,
                limit: 10,
            },
            &MemberInclusion::default(),
        )
        .await
        .unwrap();
        assert_eq!(list.len(), 2);

        // List by team + keyword.
        let list = MemberQuery::list_by_team_id(
            &mock,
            "team-1",
            Some("Ali"),
            None,
            Page {
                offset: 0,
                limit: 10,
            },
            &MemberInclusion::default(),
        )
        .await
        .unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "m-1");

        // List by team + role.
        let list = MemberQuery::list_by_team_id(
            &mock,
            "team-1",
            None,
            Some(RoleFlag::Translator),
            Page {
                offset: 0,
                limit: 10,
            },
            &MemberInclusion::default(),
        )
        .await
        .unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "m-2");
    }

    #[tokio::test]
    async fn exist_by_user_and_team_id_true() {
        let mock = MemoryMockQuery::new();
        mock.seed_member(make_member("m-1", "u-1", "team-1", RoleFlag::Admin.into()));

        let exists = MemberQuery::exist_by_user_and_team_id(&mock, "u-1", "team-1")
            .await
            .unwrap();
        assert!(exists);
    }

    #[tokio::test]
    async fn exist_by_user_and_team_id_false() {
        let mock = MemoryMockQuery::new();

        let exists = MemberQuery::exist_by_user_and_team_id(&mock, "u-1", "team-1")
            .await
            .unwrap();
        assert!(!exists);
    }

    #[tokio::test]
    async fn list_user_filter_returns_user_members() {
        let mock = MemoryMockQuery::new();
        mock.seed_member(make_member(
            "m-1",
            "user-1",
            "team-1",
            RoleFlag::Admin.into(),
        ));
        mock.seed_member(make_member(
            "m-2",
            "user-1",
            "team-2",
            RoleFlag::Translator.into(),
        ));
        mock.seed_member(make_member(
            "m-3",
            "user-2",
            "team-1",
            RoleFlag::Admin.into(),
        ));

        let list = MemberQuery::list_by_user_id(
            &mock,
            "user-1",
            Page {
                offset: 0,
                limit: 10,
            },
            &MemberInclusion::default(),
        )
        .await
        .unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn update_roles_replaces_all_roles() {
        let mock = MemoryMockQuery::new();
        mock.seed_member(make_member("m-1", "u-1", "team-1", RoleFlag::Admin.into()));

        // Replace Admin with Translator + Proofreader.
        let mut roles_bits: u32 = 0;
        roles_bits |= u32::from(RoleFlag::Translator);
        roles_bits |= u32::from(RoleFlag::Proofreader);
        let new_roles = RoleMask::try_from(roles_bits).unwrap();

        mock.transaction_scoped(|txn| {
            async move {
                let update = MemberRoleUpdate {
                    id: "m-1".into(),
                    role_mask: new_roles,
                };
                MemberQueryTransactional::update_roles(txn, &update).await
            }
            .boxed()
        })
        .await
        .unwrap();

        let found = MemberQuery::get_by_id(&mock, "m-1").await.unwrap();
        assert!(!found.has_any_role(&[RoleFlag::Admin]));
        assert!(found.has_every_role(&[RoleFlag::Translator, RoleFlag::Proofreader]));
    }

    #[tokio::test]
    async fn update_roles_missing_returns_error() {
        let mock = MemoryMockQuery::new();

        let err = mock
            .transaction_scoped(|txn| {
                async move {
                    let update = MemberRoleUpdate {
                        id: "nonexistent".into(),
                        role_mask: RoleMask::from(RoleFlag::Admin),
                    };
                    MemberQueryTransactional::update_roles(txn, &update).await
                }
                .boxed()
            })
            .await
            .err()
            .unwrap();

        assert!(is_expected_argument(&err));
    }

    #[tokio::test]
    async fn delete_removes_member() {
        let mock = MemoryMockQuery::new();
        mock.seed_member(make_member("m-1", "u-1", "team-1", RoleFlag::Admin.into()));

        mock.transaction_scoped(|txn| {
            async move { MemberQueryTransactional::delete(txn, "m-1").await }.boxed()
        })
        .await
        .unwrap();

        let snapshot = mock.snapshot();
        assert!(snapshot.members.is_empty());
    }

    #[tokio::test]
    async fn delete_missing_returns_error() {
        let mock = MemoryMockQuery::new();

        let err = mock
            .transaction_scoped(|txn| {
                async move { MemberQueryTransactional::delete(txn, "nonexistent").await }.boxed()
            })
            .await
            .err()
            .unwrap();

        assert!(is_expected_argument(&err));
    }
}
