//! Complex-domain opers for chapter entities — identity generation, workflow
//! stage transitions, pagination helpers, and permission gates.
//!
//! ## Permission model
//!
//! Read-level access (list, get) requires the caller to be a team member of the
//! owning workset's team. Write-level access (create, update info, delete) requires
//! team admin. Workflow transitions additionally validate that the caller holds a
//! role consistent with the target stage and event.

use std::borrow::Cow;
use std::collections::HashMap;

use fluent_templates::fluent_bundle::FluentValue;
use poprako_orchestra::Proxy;

use poprako_util::i18n::{trl, trl_kv};

use crate::complex::util::{
    check_user_is_team_admin, check_user_is_team_admin_with_roles,
    check_user_is_team_member,
};
use crate::model::chapter::{ChapterInfo, ChapterStageUpdate};
use crate::part::repo::oper::assignment::{
    FindAssignmentInfo, ListAssignmentInfos,
};
use crate::part::repo::oper::chapter::GetChapterInfo;
use crate::part::repo::oper::comic::GetComicInfo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};
use crate::util::next_snowflake_id;
use crate::value::chapter::{Stage, StageOper, StagePhase, try_modify_stage};
use crate::value::index::stored_index_to_user_index;
use crate::value::role::{RoleField, RoleMask};

mod cascade;

/// Domain opers for chapter entities: ID generation, workflow-stage
/// transition computation, and small pure helpers.
pub struct ChapterComplex;

impl ChapterComplex {
    /// Generate a unique, time-ordered chapter identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }

    /// Returns the user-supplied subtitle if present and non-empty, or a
    /// generated default in the format "Ch. N" (1-based).
    pub fn subtitle_or_default(subtitle: Option<String>, index: i32) -> String {
        subtitle
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| default_subtitle(index))
    }

    /// Compute the next [`ChapterStageUpdate`] by applying a [`StageOper`]
    /// to the current [`WorkflowStage`] phase of a chapter.
    pub fn build_stage_update(
        chapter_info: &ChapterInfo,
        stage: Stage,
        oper: StageOper,
    ) -> BaseResult<ChapterStageUpdate> {
        //
        let current_phase = get_phase(chapter_info, stage);

        let next_phase = try_modify_stage((stage, current_phase), oper)?;

        let chapter_stage_update = ChapterStageUpdate {
            id: chapter_info.id.clone(),
            stages: chapter_info.stages.try_set_phase(stage, next_phase)?,
        };

        accept(chapter_stage_update)
    }
}

/// Generate a human-readable default subtitle for a chapter, e.g. `"Ch. 1"`.
fn default_subtitle(index: i32) -> String {
    //
    let mut args = HashMap::new();

    args.insert(
        Cow::Borrowed("number"),
        FluentValue::String(Cow::Owned(
            stored_index_to_user_index(index).to_string(),
        )),
    );

    trl_kv("chapter-default-subtitle", &args)
}

/// Extract the current [`StagePhase`] for a given [`Stage`] from a
/// [`ChapterInfo`] record.
fn get_phase(chapter_info: &ChapterInfo, stage: Stage) -> StagePhase {
    chapter_info.stages.get_phase(stage)
}

/// Permission-gate opers for chapter entities — resolves the owning
/// team from the chapter or comic and delegates to shared team-permission
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
    ) -> BaseResult<()>
    where
        P: for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = BaseError>
            + for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        check_team_member_by_comic(proxy, user_id, comic_id).await
    }

    /// Verify the caller is a team member of the chapter's owning workset.
    pub async fn ensure_user_can_get_info<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
    ) -> BaseResult<()>
    where
        P: for<'a, 'b> Proxy<GetChapterInfo<'a, 'b>, Error = BaseError>
            + for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = BaseError>
            + for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        check_team_member_by_chapter(proxy, user_id, chapter_id).await
    }

    /// Verify the caller is a team member of the comic's owning workset
    /// (same permission level as listing — pinned chapters are visible to
    /// all team members).
    pub async fn ensure_user_can_get_pinned<P>(
        proxy: &mut P,
        user_id: &str,
        comic_id: &str,
    ) -> BaseResult<()>
    where
        P: for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = BaseError>
            + for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>
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
    ) -> BaseResult<()>
    where
        P: for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = BaseError>
            + for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        let team_id = resolve_team_id_from_comic(proxy, comic_id).await?;

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
    ) -> BaseResult<()>
    where
        P: for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = BaseError>,
    {
        check_admin(proxy, user_id, chapter_id).await
    }

    /// Verify the caller has permission to apply a workflow operation.
    pub async fn ensure_user_can_update_stage<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
        stage: Stage,
        oper: StageOper,
    ) -> BaseResult<()>
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
    ) -> BaseResult<()>
    where
        P: for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = BaseError>
            + for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        check_join_role(proxy, user_id, chapter_info, roles).await
    }

    /// Verify the caller is a team admin of the chapter's owning workset.
    pub async fn ensure_user_can_delete<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
    ) -> BaseResult<()>
    where
        P: for<'a, 'b> Proxy<GetChapterInfo<'a, 'b>, Error = BaseError>
            + for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = BaseError>
            + for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        check_team_admin_by_chapter(proxy, user_id, chapter_id).await
    }
}

/// Resolve the owning team from a comic ID, then verify the user is a team
/// member of that team.
async fn check_team_member_by_comic<P>(
    proxy: &mut P,
    user_id: &str,
    comic_id: &str,
) -> BaseResult<()>
where
    P: for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = BaseError>
        + for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>
        + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
{
    let team_id = resolve_team_id_from_comic(proxy, comic_id).await?;

    check_user_is_team_member(proxy, user_id, &team_id).await
}

/// Resolve the owning team from a chapter, then verify the user is a team member.
async fn check_team_member_by_chapter<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
) -> BaseResult<()>
where
    P: for<'a, 'b> Proxy<GetChapterInfo<'a, 'b>, Error = BaseError>
        + for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = BaseError>
        + for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>
        + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
{
    let chapter_info = proxy
        .exec(&GetChapterInfo {
            id: chapter_id,
            incls: &[],
        })
        .await?;

    check_team_member_by_comic(proxy, user_id, &chapter_info.comic_id).await
}

/// Resolve the owning team from a comic, then verify the user is a team admin.
async fn check_team_admin_by_comic<P>(
    proxy: &mut P,
    user_id: &str,
    comic_id: &str,
) -> BaseResult<()>
where
    P: for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = BaseError>
        + for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>
        + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
{
    let team_id = resolve_team_id_from_comic(proxy, comic_id).await?;

    check_user_is_team_admin(proxy, user_id, &team_id).await
}

/// Resolve the owning team from a chapter, then verify the user is a team admin.
async fn check_team_admin_by_chapter<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
) -> BaseResult<()>
where
    P: for<'a, 'b> Proxy<GetChapterInfo<'a, 'b>, Error = BaseError>
        + for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = BaseError>
        + for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>
        + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
{
    let chapter_info = proxy
        .exec(&GetChapterInfo {
            id: chapter_id,
            incls: &[],
        })
        .await?;

    check_team_admin_by_comic(proxy, user_id, &chapter_info.comic_id).await
}

/// Verify the caller is assigned as a chapter admin on this chapter.
async fn check_admin<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
) -> BaseResult<()>
where
    P: for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = BaseError>,
{
    let assignment_info = proxy
        .exec(&FindAssignmentInfo::ChapterUser {
            chapter_id,
            user_id,
        })
        .await?;

    let Some(assignment_info) = assignment_info else {
        return Err(chapter_admin_error());
    };

    if !assignment_info.roles.has_any_role(&[RoleField::ADMIN]) {
        return Err(chapter_admin_error());
    }

    accept(())
}

/// Verify the caller is permitted to perform the given workflow transition
/// on the chapter. Chapter admins bypass per-stage checks and are allowed any
/// transition. Other assignments are validated against a whitelist:
///
/// | Stage | Event | Required role |
/// |---|---|---|
/// Return the workflow roles required to perform `oper` on `stage`.
///
/// Returns an empty slice when the combination is unlisted (i.e., disallowed
/// unless the caller holds `ADMIN`).
fn required_roles_for_transition(
    stage: Stage,
    oper: StageOper,
) -> &'static [RoleField] {
    match (stage, oper) {
        //
        (Stage::RawProvide, StageOper::Advance) => &[RoleField::RAW_PROVIDER],

        (Stage::Translate, StageOper::Advance) => &[RoleField::TRANSLATOR],

        (Stage::Translate, StageOper::Revert) => {
            &[RoleField::TRANSLATOR, RoleField::PROOFREADER]
        }

        (Stage::Proofread, StageOper::Advance | StageOper::Revert) => {
            &[RoleField::PROOFREADER]
        }

        (Stage::TypesetRedraw, StageOper::Advance | StageOper::Revert) => {
            &[RoleField::TYPESETTER, RoleField::REDRAWER]
        }

        (Stage::Review, StageOper::Advance | StageOper::Revert) => {
            &[RoleField::REVIEWER]
        }

        (Stage::Publish, StageOper::Advance) => &[RoleField::PUBLISHER],

        _ => &[],
    }
}

/// Verify that at least one person on the chapter holds the role(s) required
/// for advancing `stage`. A workflow stage cannot be advanced unless someone
/// is assigned to the corresponding role.
///
/// Called only for [`StageOper::Advance`]; revert operations do not require a
/// role holder.
async fn check_chapter_has_role_holder<P>(
    proxy: &mut P,
    chapter_id: &str,
    stage: Stage,
) -> BaseResult<()>
where
    P: for<'a, 'b> Proxy<ListAssignmentInfos<'a, 'b>, Error = BaseError>,
{
    let required_roles =
        required_roles_for_transition(stage, StageOper::Advance);

    let assignment_infos = proxy
        .exec(&ListAssignmentInfos::Chapter {
            chapter_id,
            role: None,
            incls: &[],
        })
        .await?;

    let has_holder = assignment_infos
        .iter()
        .any(|info| info.roles.has_any_role(required_roles));

    if !has_holder {
        return Err(chapter_no_role_holder_error());
    }

    accept(())
}

/// | Stage | `Advance` | `Revert` |
/// |---|---|---|
/// | `RawProvide` | `RAW_PROVIDER` | - |
/// | `Translate` | `TRANSLATOR` | `PROOFREADER` |
/// | `Proofread` | `PROOFREADER` | `PROOFREADER` |
/// | `TypesetRedraw` | `TYPESETTER` or `REDRAWER` | `TYPESETTER` or `REDRAWER` |
/// | `Review` | `REVIEWER` | `REVIEWER` |
/// | `Publish` | `PUBLISHER` | - |
async fn check_workflow_role<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
    stage: Stage,
    oper: StageOper,
) -> BaseResult<()>
where
    P: for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = BaseError>
        + for<'a, 'b> Proxy<ListAssignmentInfos<'a, 'b>, Error = BaseError>,
{
    let assignment_info = proxy
        .exec(&FindAssignmentInfo::ChapterUser {
            chapter_id,
            user_id,
        })
        .await?;

    let Some(assignment_info) = assignment_info else {
        return Err(chapter_workflow_role_error());
    };

    // Domain invariant: a workflow stage cannot be advanced unless at least
    // one person on the chapter holds the required workflow role. This runs
    // before the admin bypass so that even admins must ensure the position is
    // filled.
    if oper == StageOper::Advance {
        check_chapter_has_role_holder(proxy, chapter_id, stage).await?;
    }

    let roles = assignment_info.roles;

    if roles.has_any_role(&[RoleField::ADMIN]) {
        return accept(());
    }

    let required_roles = required_roles_for_transition(stage, oper);

    if required_roles.is_empty() {
        return Err(chapter_workflow_role_error());
    }

    if !roles.has_any_role(required_roles) {
        return Err(chapter_workflow_role_error());
    }

    accept(())
}

/// Verify the caller may join a chapter with the given role mask.
///
/// Rejects `ADMIN` roles (not assignable through the join flow). The caller
/// must be a team member whose membership [`RoleMask`] contains the requested
/// role bits.
async fn check_join_role<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_info: &ChapterInfo,
    roles: RoleMask,
) -> BaseResult<()>
where
    P: for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = BaseError>
        + for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>
        + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
{
    if roles.has_any_role(&[RoleField::ADMIN]) {
        return Err(chapter_role_not_assignable_args_error());
    }

    let team_id =
        resolve_team_id_from_comic(proxy, &chapter_info.comic_id).await?;

    let member_info = proxy
        .exec(&FindMemberInfo::UserTeam {
            user_id,
            team_id: &team_id,
        })
        .await?;

    let Some(member_info) = member_info else {
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-team-member-required"),
        });
    };

    if !member_info.roles.contains_mask(roles) {
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-chapter-role-not-assignable"),
        });
    }

    accept(())
}

/// Resolve the owning team identifier by fetching a comic and its parent workset.
async fn resolve_team_id_from_comic<P>(
    proxy: &mut P,
    comic_id: &str,
) -> BaseResult<String>
where
    P: for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = BaseError>
        + for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>,
{
    let comic_info = proxy
        .exec(&GetComicInfo {
            id: comic_id,
            incls: &[],
        })
        .await?;

    let workset_info = proxy
        .exec(&GetWorksetInfo {
            id: &comic_info.workset_id,
        })
        .await?;

    accept(workset_info.team_id)
}

/// Construct a "chapter admin required" permission error.
fn chapter_admin_error() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-chapter-admin-required"),
    }
}

/// Construct a "workflow role required for this transition" permission error.
fn chapter_workflow_role_error() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-chapter-workflow-role-required"),
    }
}

/// Construct a "no one holds the required workflow role on this chapter"
/// permission error.
fn chapter_no_role_holder_error() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-chapter-no-role-holder"),
    }
}

/// Construct an "admin role not assignable through join" args error.
fn chapter_role_not_assignable_args_error() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-chapter-role-not-assignable"),
    }
}
