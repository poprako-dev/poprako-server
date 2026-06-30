//! Complex-domain opers for chapter assignments.

use poprako_util::i18n::trl;

use crate::complex::util::check_user_is_team_member;
use crate::data::assignment::UpdateAssignmentRoleData;
use crate::model::assignment::{AssignmentInfo, AssignmentListSpec, AssignmentRoleUpdate};
use crate::model::role::{RoleField, RoleMask};
use crate::part::repo::step::assignment::{AssignmentStep, GetInfoByChapterIdAndUserId};
use crate::part::repo::step::chapter::{ChapterStep, GetInfoById as ChapterGetInfoById};
use crate::part::repo::step::comic::{ComicStep, GetInfoById as ComicGetInfoById};
use crate::part::repo::step::member::{FindInfoByUserIdAndTeamId, MemberStep};
use crate::part::repo::step::user::{GetInfoById as UserGetInfoById, UserStep};
use crate::part::repo::step::workset::{GetInfoById as WorksetGetInfoById, WorksetStep};
use crate::part::shared::proxy::ProxyExecute;
use crate::result::{ExpectedVariant, RootError, RootResult, accept};
use crate::util::next_snowflake_id;

/// Domain opers for chapter assignments: ID generation and role-merge logic.
pub struct AssignmentComplex;

impl AssignmentComplex {
    /// Generate a unique assignment identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }

    /// Merge new roles into an existing assignment, preserving existing roles
    /// and writing new ones.
    pub fn merge_roles(assignment_info: &AssignmentInfo, roles: RoleMask) -> AssignmentRoleUpdate {
        AssignmentRoleUpdate {
            id: assignment_info.id.clone(),
            roles: assignment_info.roles.union(roles),
        }
    }
}

/// Permission-gate opers for chapter assignments.
pub struct AssignmentPermComplex;

impl AssignmentPermComplex {
    /// Verify the caller may list assignments selected by the list spec.
    pub async fn can_user_list_infos<P>(
        proxy: &mut P,
        user_id: &str,
        assignment_list_spec: &AssignmentListSpec,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<ChapterGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<FindInfoByUserIdAndTeamId<'a>, Error = RootError>
            + for<'a> ProxyExecute<GetInfoByChapterIdAndUserId<'a>, Error = RootError>
            + for<'a> ProxyExecute<UserGetInfoById<'a>, Error = RootError>,
    {
        match assignment_list_spec {
            AssignmentListSpec::Chapter { chapter_id, .. } => {
                check_list_by_chapter(proxy, user_id, chapter_id).await
            }
            AssignmentListSpec::User { owner_id, .. } => {
                check_list_by_user(proxy, user_id, owner_id).await
            }
        }
    }

    /// Verify the caller may mutate assignment roles with the supplied data.
    pub async fn can_user_update_roles<P>(
        proxy: &mut P,
        current_user_id: &str,
        data: &UpdateAssignmentRoleData,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<ChapterGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<FindInfoByUserIdAndTeamId<'a>, Error = RootError>
            + for<'a> ProxyExecute<GetInfoByChapterIdAndUserId<'a>, Error = RootError>,
    {
        let reviewer_result = check_reviewer(proxy, current_user_id, &data.chapter_id).await;

        if reviewer_result.is_err() {
            check_self_reduce(proxy, current_user_id, data).await?;
        }

        check_target_roles(proxy, &data.user_id, &data.chapter_id, data.roles).await
    }

    /// Verify the caller may delete the target assignment.
    pub async fn can_user_delete<P>(
        proxy: &mut P,
        current_user_id: &str,
        assignment_info: &AssignmentInfo,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<GetInfoByChapterIdAndUserId<'a>, Error = RootError>,
    {
        if current_user_id == assignment_info.user_id {
            return accept(());
        }

        check_reviewer(proxy, current_user_id, &assignment_info.chapter_id).await
    }

    /// Verify the caller is a reviewer for the target chapter.
    pub async fn can_user_review<P>(
        proxy: &mut P,
        current_user_id: &str,
        chapter_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<GetInfoByChapterIdAndUserId<'a>, Error = RootError>,
    {
        check_reviewer(proxy, current_user_id, chapter_id).await
    }

    /// Verify the target user may take the requested chapter assignment roles.
    pub async fn can_user_take_roles<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
        roles: RoleMask,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<ChapterGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<FindInfoByUserIdAndTeamId<'a>, Error = RootError>,
    {
        check_target_roles(proxy, user_id, chapter_id, roles).await
    }
}

async fn check_list_by_chapter<P>(proxy: &mut P, user_id: &str, chapter_id: &str) -> RootResult<()>
where
    P: for<'a> ProxyExecute<ChapterGetInfoById<'a>, Error = RootError>
        + for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
        + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
        + for<'a> ProxyExecute<FindInfoByUserIdAndTeamId<'a>, Error = RootError>
        + for<'a> ProxyExecute<GetInfoByChapterIdAndUserId<'a>, Error = RootError>,
{
    let team_id = resolve_team_id(proxy, chapter_id).await?;
    let member_result = check_user_is_team_member(proxy, user_id, &team_id).await;

    if member_result.is_ok() {
        return accept(());
    }

    let assignment_info = proxy
        .execute(&AssignmentStep::get_info_by_chapter_id_and_user_id(
            chapter_id, user_id,
        ))
        .await?;

    if assignment_info.is_none() {
        return Err(assignment_list_permission_error());
    }

    accept(())
}

async fn check_list_by_user<P>(
    proxy: &mut P,
    current_user_id: &str,
    owner_id: &str,
) -> RootResult<()>
where
    P: for<'a> ProxyExecute<UserGetInfoById<'a>, Error = RootError>,
{
    if current_user_id == owner_id {
        return accept(());
    }

    let user_info = proxy
        .execute(&UserStep::get_info_by_id(current_user_id))
        .await?;

    if !user_info.is_sadmin {
        return Err(assignment_list_permission_error());
    }

    accept(())
}

async fn check_reviewer<P>(proxy: &mut P, user_id: &str, chapter_id: &str) -> RootResult<()>
where
    P: for<'a> ProxyExecute<GetInfoByChapterIdAndUserId<'a>, Error = RootError>,
{
    let assignment_info = proxy
        .execute(&AssignmentStep::get_info_by_chapter_id_and_user_id(
            chapter_id, user_id,
        ))
        .await?;

    let Some(assignment_info) = assignment_info else {
        return Err(chapter_reviewer_error());
    };

    if !assignment_info.roles.has_any_role(&[RoleField::REVIEWER]) {
        return Err(chapter_reviewer_error());
    }

    accept(())
}

async fn check_self_reduce<P>(
    proxy: &mut P,
    current_user_id: &str,
    data: &UpdateAssignmentRoleData,
) -> RootResult<()>
where
    P: for<'a> ProxyExecute<GetInfoByChapterIdAndUserId<'a>, Error = RootError>,
{
    if current_user_id != data.user_id {
        return Err(assignment_self_reduce_error());
    }

    let assignment_info = proxy
        .execute(&AssignmentStep::get_info_by_chapter_id_and_user_id(
            &data.chapter_id,
            &data.user_id,
        ))
        .await?;

    let Some(assignment_info) = assignment_info else {
        return Err(assignment_self_reduce_error());
    };

    if !assignment_info.roles.contains_mask(data.roles) {
        return Err(assignment_self_reduce_error());
    }

    accept(())
}

async fn check_target_roles<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
    roles: RoleMask,
) -> RootResult<()>
where
    P: for<'a> ProxyExecute<ChapterGetInfoById<'a>, Error = RootError>
        + for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
        + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
        + for<'a> ProxyExecute<FindInfoByUserIdAndTeamId<'a>, Error = RootError>,
{
    if roles.has_any_role(&[RoleField::ADMIN]) {
        return Err(assignment_role_not_assignable_args_error());
    }

    let team_id = resolve_team_id(proxy, chapter_id).await?;
    let member_info = proxy
        .execute(&MemberStep::find_info_by_user_id_and_team_id(
            user_id, &team_id,
        ))
        .await?;

    let Some(member_info) = member_info else {
        return Err(assignment_role_not_assignable_perm_error());
    };

    if !member_info.roles.contains_mask(roles) {
        return Err(assignment_role_not_assignable_perm_error());
    }

    accept(())
}

async fn resolve_team_id<P>(proxy: &mut P, chapter_id: &str) -> RootResult<String>
where
    P: for<'a> ProxyExecute<ChapterGetInfoById<'a>, Error = RootError>
        + for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
        + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>,
{
    let chapter_info = proxy
        .execute(&ChapterStep::get_info_by_id(chapter_id))
        .await?;

    let comic_info = proxy
        .execute(&ComicStep::get_info_by_id(&chapter_info.comic_id))
        .await?;

    let workset_info = proxy
        .execute(&WorksetStep::get_info_by_id(&comic_info.workset_id))
        .await?;

    accept(workset_info.team_id)
}

fn assignment_list_permission_error() -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::PermDeny,
        message: trl("error-forbidden"),
    }
}

fn chapter_reviewer_error() -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::PermDeny,
        message: trl("error-chapter-reviewer-required"),
    }
}

fn assignment_self_reduce_error() -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::PermDeny,
        message: trl("error-forbidden"),
    }
}

fn assignment_role_not_assignable_args_error() -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::ArgsInvalid,
        message: trl("error-chapter-role-not-assignable"),
    }
}

fn assignment_role_not_assignable_perm_error() -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::PermDeny,
        message: trl("error-chapter-role-not-assignable"),
    }
}
