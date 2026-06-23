use async_trait::async_trait;
use poprako_transactional::advance::Advance;

use crate::complex::team::TeamComplex;
use crate::model::team::{TeamAvatarReservation, TeamInfo};
use crate::part::repo::Execute;
use crate::part::repo::step::team::{
    Create, Delete, GetInfoById, GetInfoExcluded, List, MarkAvatarUploaded, ReserveAvatar,
    UpdateInfo,
};
use crate::part::repo::team::{TeamRepo, TeamRepoTransactional};
use crate::part_impl::repo_mock::{
    Mock, MockContext, MockState, MockTransactional, expected, now,
};
use crate::result::RootError;

impl TeamRepo<MockContext> for Mock {}

impl TeamRepoTransactional<MockContext> for MockTransactional {}

fn mark_team_avatar_uploaded(
    state: &mut MockState,
    id: &str,
    avatar_version: i64,
) -> Result<(), RootError> {
    let team = state
        .teams
        .iter_mut()
        .find(|team| team.id == id)
        .ok_or_else(|| expected("error-team-not-found"))?;
    if team.avatar_version != avatar_version {
        return Err(expected("error-stale-avatar-upload"));
    }
    team.avatar_uploaded = true;
    team.updated_at = now();
    Ok(())
}

#[async_trait]
impl<'a> Execute<Create<'a>> for Mock {
    type Error = RootError;

    async fn execute(&self, step: &Create<'a>) -> Result<TeamInfo, Self::Error> {
        let mut state = self.state.lock().unwrap();
        if state.teams.iter().any(|team| team.id == step.form.id) {
            Err(expected("error-already-exists"))
        } else {
            let time = now();
            let team = TeamInfo {
                id: step.form.id.clone(),
                name: step.form.name.clone(),
                description: step.form.description.clone(),
                avatar_key: None,
                avatar_uploaded: false,
                avatar_version: 0,
                workset_next_index: 0,
                created_at: time,
                updated_at: time,
            };
            state.teams.push(team.clone());
            Ok(team)
        }
    }
}

#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for Mock {
    type Error = RootError;

    async fn execute(&self, step: &GetInfoById<'a>) -> Result<TeamInfo, Self::Error> {
        let state = self.state.lock().unwrap();
        state
            .teams
            .iter()
            .find(|team| team.id == step.id)
            .cloned()
            .ok_or_else(|| expected("error-team-not-found"))
    }
}

#[async_trait]
impl Execute<List> for Mock {
    type Error = RootError;

    async fn execute(&self, step: &List) -> Result<Vec<TeamInfo>, Self::Error> {
        let state = self.state.lock().unwrap();
        let mut teams = state.teams.clone();
        teams.sort_by(|left, right| right.created_at.cmp(&left.created_at));

        if step.page.offset >= teams.len() {
            return Ok(Vec::new());
        }

        let end = std::cmp::min(step.page.offset + step.page.limit, teams.len());
        Ok(teams[step.page.offset..end].to_vec())
    }
}

#[async_trait]
impl<'a> Execute<UpdateInfo<'a>> for Mock {
    type Error = RootError;

    async fn execute(&self, step: &UpdateInfo<'a>) -> Result<(), Self::Error> {
        let mut state = self.state.lock().unwrap();
        let team = state
            .teams
            .iter_mut()
            .find(|team| team.id == step.id)
            .ok_or_else(|| expected("error-team-not-found"))?;
        team.name = step.name.to_string();
        team.description = step.description.to_string();
        team.updated_at = now();
        Ok(())
    }
}

#[async_trait]
impl<'a> Execute<MarkAvatarUploaded<'a>> for Mock {
    type Error = RootError;

    async fn execute(&self, step: &MarkAvatarUploaded<'a>) -> Result<(), Self::Error> {
        let mut state = self.state.lock().unwrap();
        mark_team_avatar_uploaded(&mut state, step.id, step.avatar_version)
    }
}

#[async_trait]
impl<'a> Advance<ReserveAvatar<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &ReserveAvatar<'a>,
    ) -> Result<TeamAvatarReservation, Self::Error> {
        let team = context
            .state
            .teams
            .iter_mut()
            .find(|team| team.id == step.id)
            .ok_or_else(|| expected("error-team-not-found"))?;
        let avatar_version = team.avatar_version + 1;
        let object_key = TeamComplex::gen_avatar_key(step.id, avatar_version, step.file_extension);
        let previous_object_key = team.avatar_key.clone();
        team.avatar_key = Some(object_key.clone());
        team.avatar_uploaded = false;
        team.avatar_version = avatar_version;
        team.updated_at = now();
        Ok(TeamAvatarReservation {
            object_key,
            previous_object_key,
            avatar_version,
        })
    }
}

#[async_trait]
impl<'a> Advance<MarkAvatarUploaded<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &MarkAvatarUploaded<'a>,
    ) -> Result<(), Self::Error> {
        mark_team_avatar_uploaded(&mut context.state, step.id, step.avatar_version)
    }
}

#[async_trait]
impl<'a> Advance<GetInfoExcluded<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &GetInfoExcluded<'a>,
    ) -> Result<TeamInfo, Self::Error> {
        context
            .state
            .teams
            .iter()
            .find(|team| team.id == step.id)
            .cloned()
            .ok_or_else(|| expected("error-team-not-found"))
    }
}

#[async_trait]
impl<'a> Advance<Delete<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &Delete<'a>,
    ) -> Result<(), Self::Error> {
        let pos = context
            .state
            .teams
            .iter()
            .position(|team| team.id == step.id)
            .ok_or_else(|| expected("error-team-not-found"))?;
        context.state.teams.remove(pos);
        Ok(())
    }
}
