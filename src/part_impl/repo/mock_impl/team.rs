//! Mock team repository operations.

use std::cmp::Reverse;

use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::complex::team::TeamComplex;
use crate::model::team::{
    TeamAvatarReservation, TeamEntry, TeamInfo, TeamInfoListKind,
};
use crate::part::repo::oper::team::{
    AllocTeamWorksetIndex, CreateTeam, DeleteTeam, GetTeamInfo,
    GetTeamInfoExcluded, ListTeamInfos, ReserveTeamAvatar, UpdateTeam,
};
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, expected, now,
};
use crate::result::{BaseError, BaseResult, accept};

fn create_team(
    state: &mut MockState,
    entry: &TeamEntry,
) -> BaseResult<TeamInfo> {
    //
    if state.teams.iter().any(|team_info| team_info.id == entry.id) {
        return Err(expected("error-already-exists"));
    }

    let time = now();

    let team_info = TeamInfo {
        id: entry.id.clone(),
        name: entry.name.clone(),
        description: entry.description.clone(),
        avatar_key: None,
        avatar_uploaded: false,
        avatar_version: 0,
        created_at: time,
        updated_at: time,
    };

    state.teams.push(team_info.clone());

    accept(team_info)
}

fn get_team_info(state: &MockState, id: &str) -> BaseResult<TeamInfo> {
    state
        .teams
        .iter()
        .find(|team_info| team_info.id == id)
        .cloned()
        .ok_or_else(|| expected("error-team-not-found"))
}

fn list_team_infos(
    state: &MockState,
    oper: &ListTeamInfos<'_>,
) -> Vec<TeamInfo> {
    //
    let mut team_infos = match &oper.spec.kind {
        //
        TeamInfoListKind::JoinedBy { user_id } => state
            .teams
            .iter()
            .filter(|team_info| {
                state.members.iter().any(|member_info| {
                    member_info.user_id == user_id.as_str()
                        && member_info.team_id == team_info.id
                })
            })
            .cloned()
            .collect(),

        TeamInfoListKind::All => state.teams.clone(),
    };

    team_infos.sort_by_key(|team_info| Reverse(team_info.created_at));

    let offset = oper.spec.offset as usize;

    let limit = oper.spec.limit as usize;

    match offset >= team_infos.len() {
        //
        true => Vec::new(),

        false => {
            //
            let end = std::cmp::min(offset + limit, team_infos.len());

            team_infos[offset..end].to_vec()
        }
    }
}

fn update_team(state: &mut MockState, oper: &UpdateTeam<'_>) -> BaseResult<()> {
    //
    let team_info = state
        .teams
        .iter_mut()
        .find(|team_info| match oper {
            UpdateTeam::Info { id, .. }
            | UpdateTeam::MarkAvatarUploaded { id, .. } => team_info.id == *id,
        })
        .ok_or_else(|| expected("error-team-not-found"))?;

    match oper {
        //
        UpdateTeam::Info {
            name, description, ..
        } => {
            //
            team_info.name = name.to_string();

            team_info.description = description.to_string();
        }

        UpdateTeam::MarkAvatarUploaded {
            avatar_version,
            avatar_key,
            ..
        } => {
            //
            if team_info.avatar_version != *avatar_version
                || avatar_key.is_some_and(|avatar_key| {
                    team_info.avatar_key.as_deref() != Some(avatar_key)
                })
            {
                return Err(expected("error-stale-avatar-upload"));
            }

            team_info.avatar_uploaded = true;
        }
    }

    team_info.updated_at = now();

    accept(())
}

fn reserve_team_avatar(
    state: &mut MockState,
    oper: &ReserveTeamAvatar<'_>,
) -> BaseResult<TeamAvatarReservation> {
    //
    let team_info = state
        .teams
        .iter_mut()
        .find(|team_info| team_info.id == oper.id)
        .ok_or_else(|| expected("error-team-not-found"))?;

    let avatar_version = team_info.avatar_version + 1;

    let object_key =
        TeamComplex::gen_avatar_key(oper.id, avatar_version, oper.file_ext);

    let prev_object_key = team_info.avatar_key.clone();

    team_info.avatar_key = Some(object_key.clone());

    team_info.avatar_uploaded = false;

    team_info.avatar_version = avatar_version;

    team_info.updated_at = now();

    accept(TeamAvatarReservation {
        object_key,
        prev_object_key,
        avatar_version,
    })
}

fn delete_team(state: &mut MockState, id: &str) -> BaseResult<()> {
    //
    let position = state
        .teams
        .iter()
        .position(|team_info| team_info.id == id)
        .ok_or_else(|| expected("error-team-not-found"))?;

    let deleted_team_id = state.teams[position].id.clone();

    let deleted_workset_ids = state
        .worksets
        .iter()
        .filter(|workset_info| workset_info.team_id == deleted_team_id)
        .map(|workset_info| workset_info.id.clone())
        .collect::<Vec<_>>();

    let deleted_comic_ids = state
        .comics
        .iter()
        .filter(|comic_info| {
            deleted_workset_ids.contains(&comic_info.workset_id)
        })
        .map(|comic_info| comic_info.id.clone())
        .collect::<Vec<_>>();

    let deleted_chapter_ids = state
        .chapters
        .iter()
        .filter(|chapter_info| {
            deleted_comic_ids.contains(&chapter_info.comic_id)
        })
        .map(|chapter_info| chapter_info.id.clone())
        .collect::<Vec<_>>();

    state.teams.remove(position);

    state
        .worksets
        .retain(|workset_info| workset_info.team_id != deleted_team_id);

    state
        .members
        .retain(|member_info| member_info.team_id != deleted_team_id);

    state.member_invitations.retain(|member_invitation_info| {
        member_invitation_info.team_id != deleted_team_id
    });

    state.comics.retain(|comic_info| {
        !deleted_workset_ids.contains(&comic_info.workset_id)
    });

    state.chapters.retain(|chapter_info| {
        !deleted_comic_ids.contains(&chapter_info.comic_id)
    });

    state.pages.retain(|page_info| {
        !deleted_chapter_ids.contains(&page_info.chapter_id)
    });

    state.assignments.retain(|assignment_info| {
        !deleted_chapter_ids.contains(&assignment_info.chapter_id)
    });

    accept(())
}

impl<'a> Run<CreateTeam<'a>> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &CreateTeam<'a>) -> BaseResult<TeamInfo> {
        //
        let mut state = self.state.lock().unwrap();

        create_team(&mut state, oper.entry)
    }
}

impl<'a> Run<GetTeamInfo<'a>> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &GetTeamInfo<'a>) -> BaseResult<TeamInfo> {
        //
        let state = self.state.lock().unwrap();

        match oper {
            GetTeamInfo::Id { id } => get_team_info(&state, id),
        }
    }
}

impl<'a> Run<ListTeamInfos<'a>> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &ListTeamInfos<'a>) -> BaseResult<Vec<TeamInfo>> {
        //
        let state = self.state.lock().unwrap();

        accept(list_team_infos(&state, oper))
    }
}

impl<'a> Run<UpdateTeam<'a>> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &UpdateTeam<'a>) -> BaseResult<()> {
        //
        let mut state = self.state.lock().unwrap();

        update_team(&mut state, oper)
    }
}

impl<'a> Step<CreateTeam<'a>, MockContext> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CreateTeam<'a>,
    ) -> BaseResult<TeamInfo> {
        //
        if context.create_team_failure {
            return Err(expected("failed"));
        }

        create_team(&mut context.state, oper.entry)
    }
}

impl<'a> Step<UpdateTeam<'a>, MockContext> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UpdateTeam<'a>,
    ) -> BaseResult<()> {
        update_team(&mut context.state, oper)
    }
}

impl<'a> Step<ReserveTeamAvatar<'a>, MockContext> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ReserveTeamAvatar<'a>,
    ) -> BaseResult<TeamAvatarReservation> {
        reserve_team_avatar(&mut context.state, oper)
    }
}

impl<'a> Step<GetTeamInfoExcluded<'a>, MockContext> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetTeamInfoExcluded<'a>,
    ) -> BaseResult<TeamInfo> {
        match oper {
            GetTeamInfoExcluded::Id { id } => get_team_info(&context.state, id),
        }
    }
}

impl<'a> Step<DeleteTeam<'a>, MockContext> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeleteTeam<'a>,
    ) -> BaseResult<()> {
        delete_team(&mut context.state, oper.id)
    }
}

impl<'a> Step<AllocTeamWorksetIndex<'a>, MockContext> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &AllocTeamWorksetIndex<'a>,
    ) -> BaseResult<i32> {
        //
        // verify the team exists
        context
            .state
            .teams
            .iter()
            .find(|team| team.id == oper.id)
            .ok_or_else(|| expected("error-team-not-found"))?;

        let workset_index = context
            .state
            .worksets
            .iter()
            .filter(|workset_info| workset_info.team_id == oper.id)
            .count() as i32;

        accept(workset_index)
    }
}
