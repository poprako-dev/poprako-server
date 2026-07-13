//! Mock implementation of assignment repository operations.

use poprako_orchestra::{Run, Step};

use self::incl::apply_assignment_incls;
use crate::model::assignment::{
    AssignmentEntry, AssignmentInfo, AssignmentInfoListSpec,
};
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::oper::assignment::{
    CreateAssignment, DeleteAssignments, FindAssignmentInfo, GetAssignmentInfo,
    ListAssignmentInfos, ListAssignmentInfosExcluded, UpdateAssignmentRoles,
};
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, expected, now,
};
use crate::result::{RegularError, RegularResult};
use crate::value::assignment::AssignmentInclOpt;
use crate::value::role::RoleField;

mod incl;

impl AssignmentRepo<MockContext> for Mock {}

fn find_assignment(
    state: &MockState,
    chapter_id: &str,
    user_id: &str,
) -> Option<AssignmentInfo> {
    state
        .assignments
        .iter()
        .find(|assignment_info| {
            assignment_info.chapter_id == chapter_id
                && assignment_info.user_id == user_id
        })
        .cloned()
}

fn find_assignment_by_user_and_comic(
    state: &MockState,
    user_id: &str,
    comic_id: &str,
    incls: &[AssignmentInclOpt],
) -> Option<AssignmentInfo> {
    //
    let mut assignment_infos = state
        .assignments
        .iter()
        .filter(|assignment_info| assignment_info.user_id == user_id)
        .filter(|assignment_info| {
            state.chapters.iter().any(|chapter_info| {
                chapter_info.id == assignment_info.chapter_id
                    && chapter_info.comic_id == comic_id
            })
        })
        .cloned()
        .collect::<Vec<_>>();

    assignment_infos.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| left.id.cmp(&right.id))
    });

    let mut assignment_info = assignment_infos.into_iter().next()?;

    apply_assignment_incls(state, &mut assignment_info, incls);

    Some(assignment_info)
}

fn get_assignment(
    state: &MockState,
    id: &str,
    incl_opt: &[AssignmentInclOpt],
) -> RegularResult<AssignmentInfo> {
    //
    let mut assignment_info = state
        .assignments
        .iter()
        .find(|assignment_info| assignment_info.id == id)
        .cloned()
        .ok_or_else(|| expected("error-assignment-not-found"))?;

    apply_assignment_incls(state, &mut assignment_info, incl_opt);

    Ok(assignment_info)
}

fn list_assignments(
    state: &MockState,
    spec: &AssignmentInfoListSpec,
) -> Vec<AssignmentInfo> {
    //
    let (offset, limit, incl_opt, mut assignment_infos) = match spec {
        //
        AssignmentInfoListSpec::Chapter {
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
                .filter(|assignment_info| {
                    assignment_info.chapter_id == *chapter_id
                })
                .filter(|assignment_info| {
                    role.map(|role| assignment_info.roles.has_any_role(&[role]))
                        .unwrap_or(true)
                })
                .cloned()
                .collect::<Vec<_>>(),
        ),

        AssignmentInfoListSpec::User {
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

fn list_all_assignments_by_chapter(
    state: &MockState,
    chapter_id: &str,
    role: Option<RoleField>,
    incl_opt: &[AssignmentInclOpt],
) -> Vec<AssignmentInfo> {
    //
    let mut assignment_infos = state
        .assignments
        .iter()
        .filter(|assignment_info| assignment_info.chapter_id == chapter_id)
        .filter(|assignment_info| {
            role.map(|role| assignment_info.roles.has_any_role(&[role]))
                .unwrap_or(true)
        })
        .cloned()
        .collect::<Vec<_>>();

    for assignment_info in &mut assignment_infos {
        apply_assignment_incls(state, assignment_info, incl_opt);
    }

    assignment_infos.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| left.id.cmp(&right.id))
    });

    assignment_infos
}

fn list_assignments_by_chapter_id_excluded(
    state: &MockState,
    chapter_id: &str,
) -> Vec<AssignmentInfo> {
    list_all_assignments_by_chapter(state, chapter_id, None, &[])
}

fn create_assignment(
    state: &mut MockState,
    entry: &AssignmentEntry,
) -> RegularResult<AssignmentInfo> {
    //
    if state
        .assignments
        .iter()
        .any(|assignment_info| assignment_info.id == entry.id)
    {
        return Err(expected("error-already-exists"));
    }

    if state.assignments.iter().any(|assignment_info| {
        assignment_info.chapter_id == entry.chapter_id
            && assignment_info.user_id == entry.user_id
    }) {
        return Err(expected("error-already-exists"));
    }

    let time = now();

    let assignment_info = AssignmentInfo {
        id: entry.id.clone(),
        chapter_id: entry.chapter_id.clone(),
        user_id: entry.user_id.clone(),
        user: None,
        chapter: None,
        roles: entry.roles,
        created_at: time,
        updated_at: time,
    };

    state.assignments.push(assignment_info.clone());

    Ok(assignment_info)
}

fn delete_assignment_by_id(
    state: &mut MockState,
    id: &str,
) -> RegularResult<()> {
    //
    let index = state
        .assignments
        .iter()
        .position(|assignment_info| assignment_info.id == id)
        .ok_or_else(|| expected("error-assignment-not-found"))?;

    state.assignments.remove(index);

    Ok(())
}

fn delete_assignments_by_chapter_id(
    state: &mut MockState,
    chapter_id: &str,
) -> RegularResult<()> {
    //
    state.assignments.retain(|a| a.chapter_id != chapter_id);

    Ok(())
}

impl Run<FindAssignmentInfo<'_, '_>> for Mock {
    type Error = RegularError;

    async fn run(
        &self,
        oper: &FindAssignmentInfo<'_, '_>,
    ) -> Result<Option<AssignmentInfo>, Self::Error> {
        //
        let state = self.state.lock().unwrap();

        let assignment_info = match oper {
            //
            FindAssignmentInfo::ChapterUser {
                chapter_id,
                user_id,
            } => find_assignment(&state, chapter_id, user_id),

            FindAssignmentInfo::UserComic {
                user_id,
                comic_id,
                incls,
            } => find_assignment_by_user_and_comic(
                &state, user_id, comic_id, incls,
            ),
        };

        Ok(assignment_info)
    }
}

impl Run<ListAssignmentInfos<'_, '_>> for Mock {
    type Error = RegularError;

    async fn run(
        &self,
        oper: &ListAssignmentInfos<'_, '_>,
    ) -> Result<Vec<AssignmentInfo>, Self::Error> {
        //
        let state = self.state.lock().unwrap();

        let assignment_infos = match oper {
            //
            ListAssignmentInfos::Spec { spec } => {
                list_assignments(&state, spec)
            }

            ListAssignmentInfos::Chapter {
                chapter_id,
                role,
                incls,
            } => list_all_assignments_by_chapter(
                &state, chapter_id, *role, incls,
            ),
        };

        Ok(assignment_infos)
    }
}

impl Run<GetAssignmentInfo<'_, '_>> for Mock {
    type Error = RegularError;

    async fn run(
        &self,
        oper: &GetAssignmentInfo<'_, '_>,
    ) -> Result<AssignmentInfo, Self::Error> {
        //
        let state = self.state.lock().unwrap();

        get_assignment(&state, oper.id, oper.incls)
    }
}

impl Step<FindAssignmentInfo<'_, '_>, MockContext> for Mock {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut MockContext,
        oper: &FindAssignmentInfo<'_, '_>,
    ) -> Result<Option<AssignmentInfo>, Self::Error> {
        //
        let assignment_info = match oper {
            //
            FindAssignmentInfo::ChapterUser {
                chapter_id,
                user_id,
            } => find_assignment(&context.state, chapter_id, user_id),

            FindAssignmentInfo::UserComic {
                user_id,
                comic_id,
                incls,
            } => find_assignment_by_user_and_comic(
                &context.state,
                user_id,
                comic_id,
                incls,
            ),
        };

        Ok(assignment_info)
    }
}

impl Step<ListAssignmentInfosExcluded<'_>, MockContext> for Mock {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListAssignmentInfosExcluded<'_>,
    ) -> Result<Vec<AssignmentInfo>, Self::Error> {
        match oper {
            ListAssignmentInfosExcluded::Chapter { chapter_id } => {
                Ok(list_assignments_by_chapter_id_excluded(
                    &context.state,
                    chapter_id,
                ))
            }
        }
    }
}

impl Step<ListAssignmentInfos<'_, '_>, MockContext> for Mock {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListAssignmentInfos<'_, '_>,
    ) -> Result<Vec<AssignmentInfo>, Self::Error> {
        //
        let assignment_infos = match oper {
            //
            ListAssignmentInfos::Spec { spec } => {
                list_assignments(&context.state, spec)
            }

            ListAssignmentInfos::Chapter {
                chapter_id,
                role,
                incls,
            } => list_all_assignments_by_chapter(
                &context.state,
                chapter_id,
                *role,
                incls,
            ),
        };

        Ok(assignment_infos)
    }
}

impl Step<CreateAssignment<'_>, MockContext> for Mock {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CreateAssignment<'_>,
    ) -> Result<AssignmentInfo, Self::Error> {
        create_assignment(&mut context.state, oper.entry)
    }
}

impl Step<UpdateAssignmentRoles<'_>, MockContext> for Mock {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UpdateAssignmentRoles<'_>,
    ) -> Result<AssignmentInfo, Self::Error> {
        //
        let assignment_info = context
            .state
            .assignments
            .iter_mut()
            .find(|assignment_info| assignment_info.id == oper.update.id)
            .ok_or_else(|| expected("error-assignment-not-found"))?;

        assignment_info.roles = oper.update.roles;

        assignment_info.updated_at = now();

        Ok(assignment_info.clone())
    }
}

impl Step<DeleteAssignments<'_>, MockContext> for Mock {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeleteAssignments<'_>,
    ) -> Result<(), Self::Error> {
        match oper {
            //
            DeleteAssignments::Id { id } => {
                delete_assignment_by_id(&mut context.state, id)
            }

            DeleteAssignments::Chapter { chapter_id } => {
                delete_assignments_by_chapter_id(&mut context.state, chapter_id)
            }
        }
    }
}
