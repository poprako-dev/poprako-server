use poprako_orchestra::{OperProxy as _, Proxy};

use poprako_util::i18n::trl;

use crate::complex::chapter::role::{check_join_role, check_workflow_role};
use crate::complex::util::{
    check_user_is_team_admin, check_user_is_team_admin_with_roles,
    check_user_is_team_member,
};
use crate::model::read::proj::chapter::ChapterInfo;
use crate::part::repo::oper::assignment::{
    FindAssignmentInfo, ListAssignmentInfos,
};
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::team::ResolveTeamId;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::value::chapter::{Stage, StageOper};
use crate::value::role::{RoleField, RoleMask};

/// perm-gate opers for chapter entities — resolves the owning
/// team from the chapter or comic and delegates to shared team-perm
/// helpers (`[`check_user_is_team_member`]` / `[`check_user_is_team_admin`]`).
///
/// [`check_user_is_team_member`]: crate::complex::util::check_user_is_team_member
/// [`check_user_is_team_admin`]: crate::complex::util::check_user_is_team_admin
pub struct ChapterPermComplex;

impl ChapterPermComplex {
    /// Verify the caller is a team member of the comic's owning workset.
    pub async fn ensure_user_can_list_infos<P>(
        proxy: &mut P,
        user_id: &str,
        comic_id: &str,
    ) -> BaseRest<()>
    where
        P: for<'a> Proxy<ResolveTeamId<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        check_team_member_by_comic(proxy, user_id, comic_id).await
    }

    /// Verify the caller is a team member of the chapter's owning workset.
    pub async fn ensure_user_can_get_info<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
    ) -> BaseRest<()>
    where
        P: for<'a> Proxy<ResolveTeamId<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        check_team_member_by_chapter(proxy, user_id, chapter_id).await
    }

    /// Verify the caller is a team member of the comic's owning workset
    /// (same perm level as listing — pinned chapters are visible to
    /// all team members).
    pub async fn ensure_user_can_get_pinned<P>(
        proxy: &mut P,
        user_id: &str,
        comic_id: &str,
    ) -> BaseRest<()>
    where
        P: for<'a> Proxy<ResolveTeamId<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        check_team_member_by_comic(proxy, user_id, comic_id).await
    }

    /// Verify the caller is a team admin of the comic's owning workset.
    pub async fn ensure_user_can_create<P>(
        proxy: &mut P,
        user_id: &str,
        comic_id: &str,
        preset_assignment_roles: Option<RoleMask>,
    ) -> BaseRest<()>
    where
        P: for<'a> Proxy<ResolveTeamId<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        let team_id = ResolveTeamId::Comic { id: comic_id }
            .proxy_on(proxy)
            .await?;

        check_user_is_team_admin_with_roles(
            proxy,
            user_id,
            &team_id,
            preset_assignment_roles,
        )
        .await
    }

    /// Verify the caller is assigned as a chapter admin for metadata updates.
    pub async fn ensure_user_can_update_info<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
    ) -> BaseRest<()>
    where
        P: for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = BaseError>,
    {
        check_admin(proxy, user_id, chapter_id).await
    }

    /// Verify the caller is assigned as a chapter admin for pinning.
    pub async fn ensure_user_can_mark_pinned<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
    ) -> BaseRest<()>
    where
        P: for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = BaseError>,
    {
        check_admin(proxy, user_id, chapter_id).await
    }

    /// Verify the caller has perm to apply a workflow operation.
    pub async fn ensure_user_can_update_stage<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
        stage: Stage,
        oper: StageOper,
    ) -> BaseRest<()>
    where
        P: for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = BaseError>
            + for<'a, 'b> Proxy<ListAssignmentInfos<'a, 'b>, Error = BaseError>,
    {
        check_workflow_role(proxy, user_id, chapter_id, stage, oper).await
    }

    /// Verify the caller may join a chapter with the given [`RoleMask`].
    ///
    /// The caller must be a team member whose own [`RoleMask`] (from their
    /// membership) contains the requested role. Certain roles (e.g. `ADMIN`)
    /// are excluded from the join flow entirely.
    pub async fn ensure_user_can_join<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_info: &ChapterInfo,
        roles: RoleMask,
    ) -> BaseRest<()>
    where
        P: for<'a> Proxy<ResolveTeamId<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        check_join_role(proxy, user_id, chapter_info, roles).await
    }

    /// Verify the caller is a team admin of the chapter's owning workset.
    pub async fn ensure_user_can_delete<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
    ) -> BaseRest<()>
    where
        P: for<'a> Proxy<ResolveTeamId<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        check_team_admin_by_chapter(proxy, user_id, chapter_id).await
    }
}

// Resolve the owning team from a comic ID, then verify the user is a team
// member of that team.
async fn check_team_member_by_comic<P>(
    proxy: &mut P,
    user_id: &str,
    comic_id: &str,
) -> BaseRest<()>
where
    P: for<'a> Proxy<ResolveTeamId<'a>, Error = BaseError>
        + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
{
    let team_id = ResolveTeamId::Comic { id: comic_id }
        .proxy_on(proxy)
        .await?;

    check_user_is_team_member(proxy, user_id, &team_id).await
}

// Resolve the owning team from a chapter, then verify the user is a team member.
async fn check_team_member_by_chapter<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
) -> BaseRest<()>
where
    P: for<'a> Proxy<ResolveTeamId<'a>, Error = BaseError>
        + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
{
    let team_id = ResolveTeamId::Chapter { id: chapter_id }
        .proxy_on(proxy)
        .await?;

    check_user_is_team_member(proxy, user_id, &team_id).await
}

// Resolve the owning team from a chapter, then verify the user is a team admin.
async fn check_team_admin_by_chapter<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
) -> BaseRest<()>
where
    P: for<'a> Proxy<ResolveTeamId<'a>, Error = BaseError>
        + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
{
    let team_id = ResolveTeamId::Chapter { id: chapter_id }
        .proxy_on(proxy)
        .await?;

    check_user_is_team_admin(proxy, user_id, &team_id).await
}

// Verify the caller is assigned as a chapter admin on this chapter.
async fn check_admin<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
) -> BaseRest<()>
where
    P: for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = BaseError>,
{
    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id,
        user_id,
    }
    .proxy_on(proxy)
    .await?;

    let Some(assignment_info) = assignment_info else {
        //
        let err_message = trl("error-chapter-admin-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %user_id,
            chapter_id = %chapter_id,
            "expected error: chapter admin assignment missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    };

    if !assignment_info.roles.has_any_role(&[RoleField::ADMIN]) {
        //
        let err_message = trl("error-chapter-admin-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %user_id,
            chapter_id = %chapter_id,
            assignment_roles = ?assignment_info.roles,
            "expected error: chapter admin role missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    }

    accept(())
}
