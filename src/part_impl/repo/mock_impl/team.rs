//! Mock implementations of `TeamRepo` and `TeamRepoTransactional` for in-memory testing.

use std::cmp::Reverse;

use async_trait::async_trait;

use poprako_transactional::advance::Advance;

use crate::complex::team::TeamComplex;
use crate::model::team::{TeamAvatarReservation, TeamInfo};
use crate::part::repo::step::team::{
    Create, Delete, GetInfoById, GetInfoExcluded, IncrementWorksetNextIndex,
    ListInfos, MarkAvatarUploaded, ReserveAvatar, UpdateInfo,
};
use crate::part::repo::team::{TeamRepo, TeamRepoTransactional};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, MockTransactional, expected, now,
};
use crate::result::{RegularError, RegularResult};

impl TeamRepo<MockContext> for Mock {}

impl TeamRepoTransactional<MockContext> for MockTransactional {}

fn create_team(
    state: &mut MockState,
    step: &Create<'_>,
) -> RegularResult<TeamInfo> {
    //
    if state.teams.iter().any(|team| team.id == step.form.id) {
        return Err(expected("error-already-exists"));
    }

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

/// Updates a team record to mark its avatar as uploaded, verifying the avatar version
/// to detect stale uploads.
fn mark_team_avatar_uploaded(
    state: &mut MockState,
    id: &str,
    avatar_version: i64,
) -> RegularResult<()> {
    //
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
    type Error = RegularError;

    async fn execute(
        &self,
        step: &Create<'a>,
    ) -> Result<TeamInfo, Self::Error> {
        let mut state = self.state.lock().unwrap();
        create_team(&mut state, step)
    }
}

#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for Mock {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &GetInfoById<'a>,
    ) -> Result<TeamInfo, Self::Error> {
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
impl<'a> Execute<ListInfos<'a>> for Mock {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &ListInfos<'a>,
    ) -> Result<Vec<TeamInfo>, Self::Error> {
        let state = self.state.lock().unwrap();
        let mut teams = match step.user_id {
            Some(user_id) => state
                .teams
                .iter()
                .filter(|team| {
                    state.members.iter().any(|member| {
                        member.user_id == user_id && member.team_id == team.id
                    })
                })
                .cloned()
                .collect(),
            None => state.teams.clone(),
        };
        teams.sort_by_key(|right| Reverse(right.created_at));

        let offset = step.offset as usize;
        let limit = step.limit as usize;

        if offset >= teams.len() {
            return Ok(Vec::new());
        }

        let end = std::cmp::min(offset + limit, teams.len());
        Ok(teams[offset..end].to_vec())
    }
}

#[async_trait]
impl<'a> Execute<UpdateInfo<'a>> for Mock {
    type Error = RegularError;

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
    type Error = RegularError;

    async fn execute(
        &self,
        step: &MarkAvatarUploaded<'a>,
    ) -> Result<(), Self::Error> {
        let mut state = self.state.lock().unwrap();
        mark_team_avatar_uploaded(&mut state, step.id, step.avatar_version)
    }
}

#[async_trait]
impl<'a> Advance<ReserveAvatar<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

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
        let object_key = TeamComplex::gen_avatar_key(
            step.id,
            avatar_version,
            step.file_extension,
        );
        let prev_object_key = team.avatar_key.clone();
        team.avatar_key = Some(object_key.clone());
        team.avatar_uploaded = false;
        team.avatar_version = avatar_version;
        team.updated_at = now();
        Ok(TeamAvatarReservation {
            object_key,
            prev_object_key,
            avatar_version,
        })
    }
}

#[async_trait]
impl<'a> Advance<Create<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &Create<'a>,
    ) -> Result<TeamInfo, Self::Error> {
        create_team(&mut context.state, step)
    }
}

#[async_trait]
impl<'a> Advance<MarkAvatarUploaded<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &MarkAvatarUploaded<'a>,
    ) -> Result<(), Self::Error> {
        mark_team_avatar_uploaded(
            &mut context.state,
            step.id,
            step.avatar_version,
        )
    }
}

#[async_trait]
impl<'a> Advance<GetInfoExcluded<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

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
    type Error = RegularError;

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
        let deleted_team_id = context.state.teams[pos].id.clone();

        let deleted_workset_ids = context
            .state
            .worksets
            .iter()
            .filter(|workset| workset.team_id == deleted_team_id)
            .map(|workset| workset.id.clone())
            .collect::<Vec<_>>();
        let deleted_comic_ids = context
            .state
            .comics
            .iter()
            .filter(|comic| {
                deleted_workset_ids
                    .iter()
                    .any(|workset_id| workset_id == &comic.workset_id)
            })
            .map(|comic| comic.id.clone())
            .collect::<Vec<_>>();
        let deleted_chapter_ids = context
            .state
            .chapters
            .iter()
            .filter(|chapter_info| {
                deleted_comic_ids
                    .iter()
                    .any(|comic_id| comic_id == &chapter_info.comic_id)
            })
            .map(|chapter_info| chapter_info.id.clone())
            .collect::<Vec<_>>();

        context.state.teams.remove(pos);
        context
            .state
            .worksets
            .retain(|workset| workset.team_id != deleted_team_id);
        context
            .state
            .members
            .retain(|member| member.team_id != deleted_team_id);
        context
            .state
            .member_invitations
            .retain(|member_invitation| {
                member_invitation.team_id != deleted_team_id
            });
        context.state.comics.retain(|comic| {
            !deleted_workset_ids
                .iter()
                .any(|workset_id| workset_id == &comic.workset_id)
        });
        context.state.chapters.retain(|chapter_info| {
            !deleted_comic_ids
                .iter()
                .any(|comic_id| comic_id == &chapter_info.comic_id)
        });
        context.state.pages.retain(|page_info| {
            !deleted_chapter_ids
                .iter()
                .any(|chapter_id| chapter_id == &page_info.chapter_id)
        });
        context.state.assignments.retain(|assignment_info| {
            !deleted_chapter_ids
                .iter()
                .any(|chapter_id| chapter_id == &assignment_info.chapter_id)
        });
        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<IncrementWorksetNextIndex<'a>, MockContext>
    for MockTransactional
{
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &IncrementWorksetNextIndex<'a>,
    ) -> Result<i32, Self::Error> {
        let team = context
            .state
            .teams
            .iter_mut()
            .find(|team| team.id == step.id)
            .ok_or_else(|| expected("error-team-not-found"))?;
        let index = team.workset_next_index;
        team.workset_next_index += 1;
        team.updated_at = now();
        Ok(index)
    }
}
