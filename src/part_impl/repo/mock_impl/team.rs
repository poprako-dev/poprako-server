//! Mock team repository operations.

// In-memory team-ownership projections.
mod resolve;

use std::cmp::Reverse;

use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::complex::team::TeamComplex;
use crate::model::read::proj::team::TeamInfo;
use crate::model::write::team::{TeamAvatarReservation, TeamEntry};
use crate::part::nucl::RepeatableRead;
use crate::part::repo::oper::team::{
    AllocTeamWorksetIndex, CreateTeam, DeleteTeam, GetTeamInfo,
    GetTeamInfoExcluded, ListTeamInfos, LockTeam, ReserveTeamAvatar,
    UpdateTeam,
};
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, expected, now,
};
use crate::result::{BaseError, BaseRest, accept};

// Internal implementation of `create_team`.
fn create_team(state: &mut MockState, entry: &TeamEntry) -> BaseRest<TeamInfo> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    if state.teams.iter().any(|team_info| team_info.id == entry.id) {
        return Err(expected("error-already-exists"));
    }

    let time = now();

    let team_info = TeamInfo {
        id: entry.id.clone(),
        name: entry.name.clone(),
        description: entry.description.clone(),
        avatar_key: None,
        is_avatar_uploaded: None,
        avatar_version: None,
        avatar_hash: None,
        avatar_ext: None,
        created_at: time,
        updated_at: time,
    };

    state.teams.push(team_info.clone());

    accept(team_info)
}

// Internal implementation of `get_team_info`.
fn get_team_info(state: &MockState, id: &str) -> BaseRest<TeamInfo> {
    //
    state
        .teams
        .iter()
        .find(|team_info| team_info.id == id)
        .cloned()
        .ok_or_else(|| expected("error-team-not-found"))
}

// Internal implementation of `list_team_infos`.
fn list_team_infos(
    state: &MockState,
    oper: &ListTeamInfos<'_>,
) -> Vec<TeamInfo> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    let mut team_infos = match oper.spec.user_id.as_deref() {
        //
        Some(user_id) => state
            .teams
            .iter()
            .filter(|team_info| {
                //
                state.members.iter().any(|member_info| {
                    //
                    member_info.user_id == user_id
                        && member_info.team_id == team_info.id
                })
            })
            .cloned()
            .collect(),

        None => state.teams.clone(),
    };

    team_infos.sort_by_key(|team_info| Reverse(team_info.created_at));

    let offset = oper.spec.offset as usize;

    let limit = oper.spec.limit as usize;

    match offset >= team_infos.len() {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        true => Vec::new(),

        false => {
            //
            // Internal implementation detail.
            // Internal implementation detail.
            let end = std::cmp::min(offset + limit, team_infos.len());

            team_infos[offset..end].to_vec()
        }
    }
}

// Internal implementation of `update_team`.
fn update_team(state: &mut MockState, oper: &UpdateTeam<'_>) -> BaseRest<()> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    let id = match oper {
        //
        UpdateTeam::Info { repl } => repl.id.as_str(),

        UpdateTeam::MarkAvatarUploaded { repl } => repl.id.as_str(),
    };

    let team_info = state
        .teams
        .iter_mut()
        .find(|team_info| team_info.id == id)
        .ok_or_else(|| expected("error-team-not-found"))?;

    match oper {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        UpdateTeam::Info { repl } => {
            //
            // Internal implementation detail.
            // Internal implementation detail.
            team_info.name = repl.name.clone();

            team_info.description = repl.description.clone();
        }

        UpdateTeam::MarkAvatarUploaded { repl } => {
            //
            // Internal implementation detail.
            // Internal implementation detail.
            if team_info.avatar_version != Some(repl.avatar_version)
                || repl.avatar_key.as_deref().is_some_and(|avatar_key| {
                    team_info.avatar_key.as_deref() != Some(avatar_key)
                })
            {
                return Err(expected("error-stale-avatar-upload"));
            }

            team_info.is_avatar_uploaded = Some(repl.is_avatar_uploaded);
        }
    }

    team_info.updated_at = now();

    accept(())
}

// Internal implementation of `reserve_team_avatar`.
fn reserve_team_avatar(
    state: &mut MockState,
    oper: &ReserveTeamAvatar<'_>,
) -> BaseRest<TeamAvatarReservation> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    let team_info = state
        .teams
        .iter_mut()
        .find(|team_info| team_info.id == oper.id)
        .ok_or_else(|| expected("error-team-not-found"))?;

    let same_hash = team_info.avatar_key.is_some()
        && team_info.avatar_hash.as_ref() == Some(oper.image_hash);

    if same_hash && team_info.avatar_ext != Some(oper.image_ext) {
        return Err(expected("error-invalid-image-extension"));
    }

    if same_hash {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let object_key = team_info.avatar_key.clone().ok_or_else(|| {
            //
            BaseError::Unrecoverable {
                message: "[reserve_team_avatar] avatar key is missing".into(),
            }
        })?;

        return accept(TeamAvatarReservation {
            object_key,
            prev_object_key: None,
            avatar_version: team_info.avatar_version.ok_or_else(|| {
                //
                BaseError::Unrecoverable {
                    message: "[reserve_team_avatar] avatar version is missing"
                        .into(),
                }
            })?,
            is_upload_required: team_info.is_avatar_uploaded != Some(true),
        });
    }

    let avatar_version = team_info
        .avatar_version
        .unwrap_or(0)
        .checked_add(1)
        .ok_or_else(|| BaseError::Unrecoverable {
        message: "[reserve_team_avatar] avatar version overflow".into(),
    })?;

    let object_key = TeamComplex::gen_avatar_key(
        oper.id,
        avatar_version,
        oper.image_ext.suffix(),
    );

    let prev_object_key = team_info.avatar_key.clone();

    team_info.avatar_key = Some(object_key.clone());

    team_info.is_avatar_uploaded = Some(false);

    team_info.avatar_version = Some(avatar_version);

    team_info.avatar_hash = Some(oper.image_hash.clone());

    team_info.avatar_ext = Some(oper.image_ext);

    team_info.updated_at = now();

    accept(TeamAvatarReservation {
        object_key,
        prev_object_key,
        avatar_version,
        is_upload_required: true,
    })
}

// Internal implementation of `delete_team`.
fn delete_team(state: &mut MockState, id: &str) -> BaseRest<()> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
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
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(&self, oper: &CreateTeam<'a>) -> BaseRest<TeamInfo> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let mut state = self.state.lock().unwrap();

        create_team(&mut state, oper.entry)
    }
}

impl<'a> Run<GetTeamInfo<'a>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(&self, oper: &GetTeamInfo<'a>) -> BaseRest<TeamInfo> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        match oper {
            GetTeamInfo::Id { id } => get_team_info(&state, id),
        }
    }
}

impl<'a> Run<ListTeamInfos<'a>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(&self, oper: &ListTeamInfos<'a>) -> BaseRest<Vec<TeamInfo>> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        accept(list_team_infos(&state, oper))
    }
}

impl<'a> Run<UpdateTeam<'a>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(&self, oper: &UpdateTeam<'a>) -> BaseRest<()> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let mut state = self.state.lock().unwrap();

        update_team(&mut state, oper)
    }
}

impl<'a> Step<CreateTeam<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CreateTeam<'a>,
    ) -> BaseRest<TeamInfo> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        if context.create_team_failure {
            return Err(expected("failed"));
        }

        create_team(&mut context.state, oper.entry)
    }
}

impl<'a> Step<UpdateTeam<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UpdateTeam<'a>,
    ) -> BaseRest<()> {
        update_team(&mut context.state, oper)
    }
}

impl<'a> Step<ReserveTeamAvatar<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ReserveTeamAvatar<'a>,
    ) -> BaseRest<TeamAvatarReservation> {
        reserve_team_avatar(&mut context.state, oper)
    }
}

impl<'a> Step<GetTeamInfoExcluded<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetTeamInfoExcluded<'a>,
    ) -> BaseRest<TeamInfo> {
        //
        match oper {
            GetTeamInfoExcluded::Id { id } => get_team_info(&context.state, id),
        }
    }
}

impl<'a> Step<LockTeam<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &LockTeam<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        get_team_info(&context.state, oper.id)?;

        accept(())
    }
}

impl<'a> Step<DeleteTeam<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeleteTeam<'a>,
    ) -> BaseRest<()> {
        delete_team(&mut context.state, oper.id)
    }
}

impl<'a> Step<AllocTeamWorksetIndex<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &AllocTeamWorksetIndex<'a>,
    ) -> BaseRest<i32> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
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
