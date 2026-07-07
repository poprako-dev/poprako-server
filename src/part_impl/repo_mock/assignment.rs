//! Mock implementations of `AssignmentRepo` and `AssignmentRepoTransactional`.

use async_trait::async_trait;

use poprako_transactional::advance::Advance;

use crate::model::assignment::{AssignmentForm, AssignmentInfo, AssignmentListSpec};
use crate::model::chapter::ChapterInfo;
use crate::model::comic::ComicInfo;
use crate::model::team::TeamInfo;
use crate::model::user::UserInfo;
use crate::model::workset::WorksetInfo;
use crate::part::repo::assignment::{AssignmentRepo, AssignmentRepoTransactional};
use crate::part::repo::step::assignment::{
    Create, Delete, GetInfoByChapterIdAndUserId, GetInfoById, ListInfos,
    ListInfosByChapterIdExcluded, PutRoles,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo_mock::{Mock, MockContext, MockState, MockTransactional, expected, now};
use crate::result::{RegularError, RegularResult};
use crate::value::assignment::AssignmentInclOpt;
use crate::value::incl::expand_incl_opts;

impl AssignmentRepo<MockContext> for Mock {}

impl AssignmentRepoTransactional<MockContext> for MockTransactional {}

fn find_user(state: &MockState, user_id: &str) -> Option<UserInfo> {
    state
        .users
        .iter()
        .find(|user_info| user_info.id == user_id)
        .cloned()
}

fn find_chapter(state: &MockState, chapter_id: &str) -> Option<ChapterInfo> {
    let mut chapter_info = state
        .chapters
        .iter()
        .find(|chapter_info| chapter_info.id == chapter_id)
        .cloned()?;

    chapter_info.comic = None;
    chapter_info.creator = None;

    Some(chapter_info)
}

fn find_comic(state: &MockState, comic_id: &str) -> Option<ComicInfo> {
    let mut comic_info = state
        .comics
        .iter()
        .find(|comic_info| comic_info.id == comic_id)
        .cloned()?;

    comic_info.workset = None;
    comic_info.team = None;
    comic_info.creator = None;

    Some(comic_info)
}

fn find_workset(state: &MockState, workset_id: &str) -> Option<WorksetInfo> {
    state
        .worksets
        .iter()
        .find(|workset_info| workset_info.id == workset_id)
        .cloned()
}

fn find_team_for_workset(state: &MockState, workset_info: &WorksetInfo) -> Option<TeamInfo> {
    state
        .teams
        .iter()
        .find(|team_info| team_info.id == workset_info.team_id)
        .cloned()
}

fn apply_user_incl(state: &MockState, assignment_info: &mut AssignmentInfo, include_user: bool) {
    if include_user {
        assignment_info.user = find_user(state, &assignment_info.user_id);
    }
}

fn apply_chapter_incl(
    state: &MockState,
    assignment_info: &mut AssignmentInfo,
    include_chapter: bool,
) {
    if include_chapter {
        assignment_info.chapter = find_chapter(state, &assignment_info.chapter_id);
    }
}

fn apply_chapter_comic_incl(
    state: &MockState,
    assignment_info: &mut AssignmentInfo,
    include_comic: bool,
) {
    if !include_comic {
        return;
    }

    let Some(chapter_info) = &mut assignment_info.chapter else {
        return;
    };

    chapter_info.comic = find_comic(state, &chapter_info.comic_id);
}

fn apply_chapter_comic_workset_incl(
    state: &MockState,
    assignment_info: &mut AssignmentInfo,
    include_workset: bool,
) {
    if !include_workset {
        return;
    }

    let Some(chapter_info) = &mut assignment_info.chapter else {
        return;
    };

    let Some(comic_info) = &mut chapter_info.comic else {
        return;
    };

    comic_info.workset = find_workset(state, &comic_info.workset_id);
}

fn apply_chapter_comic_workset_team_incl(
    state: &MockState,
    assignment_info: &mut AssignmentInfo,
    include_team: bool,
) {
    if !include_team {
        return;
    }

    let Some(chapter_info) = &mut assignment_info.chapter else {
        return;
    };

    let Some(comic_info) = &mut chapter_info.comic else {
        return;
    };

    let Some(workset_info) = &comic_info.workset else {
        return;
    };

    comic_info.team = find_team_for_workset(state, workset_info);
}

fn apply_chapter_creator_incl(
    state: &MockState,
    assignment_info: &mut AssignmentInfo,
    include_creator: bool,
) {
    if !include_creator {
        return;
    }

    let Some(chapter_info) = &mut assignment_info.chapter else {
        return;
    };

    chapter_info.creator = find_user(state, &chapter_info.creator_id);
}

fn apply_chapter_comic_creator_incl(
    state: &MockState,
    assignment_info: &mut AssignmentInfo,
    include_creator: bool,
) {
    if !include_creator {
        return;
    }

    let Some(chapter_info) = &mut assignment_info.chapter else {
        return;
    };

    let Some(comic_info) = &mut chapter_info.comic else {
        return;
    };

    comic_info.creator = find_user(state, &comic_info.creator_id);
}

fn apply_assignment_incls(
    state: &MockState,
    assignment_info: &mut AssignmentInfo,
    incl_opt: &[AssignmentInclOpt],
) {
    assignment_info.user = None;
    assignment_info.chapter = None;

    for incl_opt in expand_incl_opts(incl_opt) {
        match incl_opt {
            AssignmentInclOpt::User => apply_user_incl(state, assignment_info, true),
            AssignmentInclOpt::Chapter => apply_chapter_incl(state, assignment_info, true),
            AssignmentInclOpt::ChapterComic => {
                apply_chapter_comic_incl(state, assignment_info, true)
            }
            AssignmentInclOpt::ChapterComicWorkset => {
                apply_chapter_comic_workset_incl(state, assignment_info, true)
            }
            AssignmentInclOpt::ChapterComicWorksetTeam => {
                apply_chapter_comic_workset_team_incl(state, assignment_info, true)
            }
            AssignmentInclOpt::ChapterCreator => {
                apply_chapter_creator_incl(state, assignment_info, true)
            }
            AssignmentInclOpt::ChapterComicCreator => {
                apply_chapter_comic_creator_incl(state, assignment_info, true)
            }
        }
    }
}

fn find_assignment(state: &MockState, chapter_id: &str, user_id: &str) -> Option<AssignmentInfo> {
    state
        .assignments
        .iter()
        .find(|assignment_info| {
            assignment_info.chapter_id == chapter_id && assignment_info.user_id == user_id
        })
        .cloned()
}

fn get_assignment(
    state: &MockState,
    id: &str,
    incl_opt: &[AssignmentInclOpt],
) -> RegularResult<AssignmentInfo> {
    let mut assignment_info = state
        .assignments
        .iter()
        .find(|assignment_info| assignment_info.id == id)
        .cloned()
        .ok_or_else(|| expected("error-assignment-not-found"))?;

    apply_assignment_incls(state, &mut assignment_info, incl_opt);

    Ok(assignment_info)
}

fn list_assignments(state: &MockState, spec: &AssignmentListSpec) -> Vec<AssignmentInfo> {
    let (offset, limit, incl_opt, mut assignment_infos) = match spec {
        AssignmentListSpec::Chapter {
            chapter_id,
            role,
            incl_opt,
            offset,
            limit,
        } => (
            *offset,
            *limit,
            incl_opt,
            state
                .assignments
                .iter()
                .filter(|assignment_info| assignment_info.chapter_id == *chapter_id)
                .filter(|assignment_info| {
                    role.map(|role| assignment_info.roles.has_any_role(&[role]))
                        .unwrap_or(true)
                })
                .cloned()
                .collect::<Vec<_>>(),
        ),
        AssignmentListSpec::User {
            owner_id,
            role,
            incl_opt,
            offset,
            limit,
        } => (
            *offset,
            *limit,
            incl_opt,
            state
                .assignments
                .iter()
                .filter(|assignment_info| assignment_info.user_id == *owner_id)
                .filter(|assignment_info| {
                    role.map(|role| assignment_info.roles.has_any_role(&[role]))
                        .unwrap_or(true)
                })
                .cloned()
                .collect::<Vec<_>>(),
        ),
    };

    for assignment_info in &mut assignment_infos {
        apply_assignment_incls(state, assignment_info, incl_opt);
    }

    assignment_infos.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| left.id.cmp(&right.id))
    });

    let offset = offset as usize;
    let limit = limit as usize;

    if offset >= assignment_infos.len() {
        return Vec::new();
    }

    let end = std::cmp::min(offset + limit, assignment_infos.len());
    assignment_infos[offset..end].to_vec()
}

fn list_assignments_by_chapter_id_excluded(
    state: &MockState,
    chapter_id: &str,
) -> Vec<AssignmentInfo> {
    state
        .assignments
        .iter()
        .filter(|assignment_info| assignment_info.chapter_id == chapter_id)
        .cloned()
        .collect()
}

fn create_assignment(
    state: &mut MockState,
    form: &AssignmentForm,
) -> RegularResult<AssignmentInfo> {
    if state
        .assignments
        .iter()
        .any(|assignment_info| assignment_info.id == form.id)
    {
        return Err(expected("error-already-exists"));
    }
    if state.assignments.iter().any(|assignment_info| {
        assignment_info.chapter_id == form.chapter_id && assignment_info.user_id == form.user_id
    }) {
        return Err(expected("error-already-exists"));
    }

    let time = now();
    let assignment_info = AssignmentInfo {
        id: form.id.clone(),
        chapter_id: form.chapter_id.clone(),
        user_id: form.user_id.clone(),
        user: None,
        chapter: None,
        roles: form.roles,
        created_at: time,
        updated_at: time,
    };
    state.assignments.push(assignment_info.clone());
    Ok(assignment_info)
}

fn delete_assignment_by_id(state: &mut MockState, id: &str) -> RegularResult<()> {
    let index = state
        .assignments
        .iter()
        .position(|assignment_info| assignment_info.id == id)
        .ok_or_else(|| expected("error-assignment-not-found"))?;
    state.assignments.remove(index);
    Ok(())
}

#[async_trait]
impl<'a> Execute<GetInfoByChapterIdAndUserId<'a>> for Mock {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &GetInfoByChapterIdAndUserId<'a>,
    ) -> Result<Option<AssignmentInfo>, Self::Error> {
        let state = self.state.lock().unwrap();
        Ok(find_assignment(&state, step.chapter_id, step.user_id))
    }
}

#[async_trait]
impl<'a> Execute<ListInfos<'a>> for Mock {
    type Error = RegularError;

    async fn execute(&self, step: &ListInfos<'a>) -> Result<Vec<AssignmentInfo>, Self::Error> {
        let state = self.state.lock().unwrap();
        Ok(list_assignments(&state, step.spec))
    }
}

#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for Mock {
    type Error = RegularError;

    async fn execute(&self, step: &GetInfoById<'a>) -> Result<AssignmentInfo, Self::Error> {
        let state = self.state.lock().unwrap();

        get_assignment(&state, step.id, step.incl_opt)
    }
}

#[async_trait]
impl<'a> Advance<GetInfoByChapterIdAndUserId<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &GetInfoByChapterIdAndUserId<'a>,
    ) -> Result<Option<AssignmentInfo>, Self::Error> {
        Ok(find_assignment(
            &context.state,
            step.chapter_id,
            step.user_id,
        ))
    }
}

#[async_trait]
impl<'a> Advance<ListInfosByChapterIdExcluded<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &ListInfosByChapterIdExcluded<'a>,
    ) -> Result<Vec<AssignmentInfo>, Self::Error> {
        Ok(list_assignments_by_chapter_id_excluded(
            &context.state,
            step.chapter_id,
        ))
    }
}

#[async_trait]
impl<'a> Advance<Create<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &Create<'a>,
    ) -> Result<AssignmentInfo, Self::Error> {
        create_assignment(&mut context.state, step.form)
    }
}

#[async_trait]
impl<'a> Advance<PutRoles<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &PutRoles<'a>,
    ) -> Result<AssignmentInfo, Self::Error> {
        let assignment_info = context
            .state
            .assignments
            .iter_mut()
            .find(|assignment_info| assignment_info.id == step.update.id)
            .ok_or_else(|| expected("error-assignment-not-found"))?;

        assignment_info.roles = step.update.roles;
        assignment_info.updated_at = now();
        Ok(assignment_info.clone())
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
        delete_assignment_by_id(&mut context.state, step.id)
    }
}
