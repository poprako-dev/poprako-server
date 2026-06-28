//! Complex-domain operations for chapter entities — identity generation, workflow
//! stage transitions, pagination helpers, and permission gates.
//!
//! ## Permission model
//!
//! Read-level access (list, get) requires the caller to be a team member of the
//! owning workset's team. Write-level access (create, update info, delete) requires
//! team admin. Workflow transitions additionally validate that the caller holds a
//! role consistent with the target stage and event.

use poprako_util::i18n::trl;

use crate::complex::util::{check_user_is_team_admin, check_user_is_team_member};
use crate::model::chapter::{ChapterInfo, ChapterStageUpdate};
use crate::model::role::{RoleField, RoleMask};
use crate::part::repo::proxy::ProxyExecute;
use crate::part::repo::step::assignment::{AssignmentStep, GetByChapterUserId};
use crate::part::repo::step::chapter::{ChapterStep, GetInfoById as ChapterGetInfoById};
use crate::part::repo::step::comic::{ComicStep, GetInfoById as ComicGetInfoById};
use crate::part::repo::step::member::{FindByUserTeamId, MemberStep};
use crate::part::repo::step::workset::{GetInfoById as WorksetGetInfoById, WorksetStep};
use crate::result::{ExpectedVariant, RootError, RootResult, accept};
use crate::util::next_snowflake_id;
use crate::value::chapter::{StagePhase, WorkflowEvent, WorkflowStage, try_modify_stage};

/// Domain operations for chapter entities: ID generation, workflow-stage
/// transition computation, and small pure helpers.
pub struct ChapterComplex;

impl ChapterComplex {
    /// Generate a unique, time-ordered chapter identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }

    /// Returns the user-supplied subtitle if present and non-empty, or a
    /// generated default in the format "第 N 话" (1-based).
    pub fn subtitle_or_default(subtitle: Option<String>, index: i32) -> String {
        subtitle
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| default_subtitle(index))
    }

    /// Compute the next [`ChapterStageUpdate`] by applying a [`WorkflowEvent`]
    /// to the current [`WorkflowStage`] phase of a chapter.
    ///
    /// Delegates the transition legality check to [`try_modify_stage`] and
    /// returns a `ChapterStageUpdate` with exactly one non-`None` phase field
    /// (the one being advanced/reverted).
    pub fn build_stage_update(
        chapter_info: &ChapterInfo,
        stage: WorkflowStage,
        event: WorkflowEvent,
    ) -> RootResult<ChapterStageUpdate> {
        let current_phase = get_phase(chapter_info, stage);
        let next_phase = try_modify_stage((stage, current_phase), event)?;

        let mut chapter_stage_update = ChapterStageUpdate {
            id: chapter_info.id.clone(),
            raw_provide_phase: None,
            translate_phase: None,
            proofread_phase: None,
            typeset_redraw_phase: None,
            review_phase: None,
            publish_phase: None,
        };

        match stage {
            WorkflowStage::RawProvide => chapter_stage_update.raw_provide_phase = Some(next_phase),
            WorkflowStage::Translate => chapter_stage_update.translate_phase = Some(next_phase),
            WorkflowStage::Proofread => chapter_stage_update.proofread_phase = Some(next_phase),
            WorkflowStage::TypesetRedraw => {
                chapter_stage_update.typeset_redraw_phase = Some(next_phase);
            }
            WorkflowStage::Review => chapter_stage_update.review_phase = Some(next_phase),
            WorkflowStage::Publish => chapter_stage_update.publish_phase = Some(next_phase),
        }

        accept(chapter_stage_update)
    }
}

/// Generate a human-readable default subtitle for a chapter, e.g. "第 1 话".
fn default_subtitle(index: i32) -> String {
    // FIXME: trl_kv
    format!("第 {} 话", index + 1)
}

/// Extract the current [`StagePhase`] for a given [`WorkflowStage`] from a
/// [`ChapterInfo`] record.
fn get_phase(chapter_info: &ChapterInfo, stage: WorkflowStage) -> StagePhase {
    chapter_info.stages.get_phase(stage)
}

/// Permission-gate operations for chapter entities — resolves the owning
/// team from the chapter or comic and delegates to shared team-permission
/// helpers (`[`check_user_is_team_member`]` / `[`check_user_is_team_admin`]`).
///
/// [`check_user_is_team_member`]: crate::complex::util::check_user_is_team_member
/// [`check_user_is_team_admin`]: crate::complex::util::check_user_is_team_admin
pub struct ChapterPermComplex;

impl ChapterPermComplex {
    /// Verify the caller is a team member of the comic's owning workset.
    pub async fn can_user_list_infos<P>(
        proxy: &mut P,
        user_id: &str,
        comic_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        check_team_member_by_comic(proxy, user_id, comic_id).await
    }

    /// Verify the caller is a team member of the chapter's owning workset.
    pub async fn can_user_get_info<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<ChapterGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        check_team_member_by_chapter(proxy, user_id, chapter_id).await
    }

    /// Verify the caller is a team member of the comic's owning workset
    /// (same permission level as listing — pinned chapters are visible to
    /// all team members).
    pub async fn can_user_get_pinned<P>(
        proxy: &mut P,
        user_id: &str,
        comic_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        check_team_member_by_comic(proxy, user_id, comic_id).await
    }

    /// Verify the caller is a team admin of the comic's owning workset.
    pub async fn can_user_create<P>(proxy: &mut P, user_id: &str, comic_id: &str) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        check_team_admin_by_comic(proxy, user_id, comic_id).await
    }

    /// Verify the caller has permission to update a chapter.
    ///
    /// When `workflow` is provided, delegates to [`check_workflow_role`] which
    /// verifies the caller holds a role consistent with the target
    /// stage/event. Without `workflow`, falls back to [`check_reviewer`]
    /// (the caller must be assigned as a reviewer).
    pub async fn can_user_update_info<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
        workflow: Option<(WorkflowStage, WorkflowEvent)>,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<GetByChapterUserId<'a>, Error = RootError>,
    {
        if let Some((stage, event)) = workflow {
            return check_workflow_role(proxy, user_id, chapter_id, stage, event).await;
        }

        check_reviewer(proxy, user_id, chapter_id).await
    }

    /// Verify the caller may join a chapter with the given [`RoleMask`].
    ///
    /// The caller must be a team member whose own [`RoleMask`] (from their
    /// membership) contains the requested role. Certain roles (e.g. `ADMIN`)
    /// are excluded from the join flow entirely.
    pub async fn can_user_join<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_info: &ChapterInfo,
        role_mask: RoleMask,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        check_join_role(proxy, user_id, chapter_info, role_mask).await
    }

    /// Verify the caller is a team admin of the chapter's owning workset.
    pub async fn can_user_delete<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<ChapterGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        check_team_admin_by_chapter(proxy, user_id, chapter_id).await
    }
}

/// Resolve the owning team from a comic, then verify the user is a team member.
async fn check_team_member_by_comic<P>(
    proxy: &mut P,
    user_id: &str,
    comic_id: &str,
) -> RootResult<()>
where
    P: for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
        + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
        + for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
{
    let team_id = resolve_team_id_from_comic(proxy, comic_id).await?;

    check_user_is_team_member(proxy, user_id, &team_id).await
}

/// Resolve the owning team from a chapter, then verify the user is a team member.
async fn check_team_member_by_chapter<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
) -> RootResult<()>
where
    P: for<'a> ProxyExecute<ChapterGetInfoById<'a>, Error = RootError>
        + for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
        + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
        + for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
{
    let chapter_info = proxy
        .execute(&ChapterStep::get_info_by_id(chapter_id))
        .await?;

    check_team_member_by_comic(proxy, user_id, &chapter_info.comic_id).await
}

/// Resolve the owning team from a comic, then verify the user is a team admin.
async fn check_team_admin_by_comic<P>(
    proxy: &mut P,
    user_id: &str,
    comic_id: &str,
) -> RootResult<()>
where
    P: for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
        + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
        + for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
{
    let team_id = resolve_team_id_from_comic(proxy, comic_id).await?;

    check_user_is_team_admin(proxy, user_id, &team_id).await
}

/// Resolve the owning team from a chapter, then verify the user is a team admin.
async fn check_team_admin_by_chapter<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
) -> RootResult<()>
where
    P: for<'a> ProxyExecute<ChapterGetInfoById<'a>, Error = RootError>
        + for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
        + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
        + for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
{
    let chapter_info = proxy
        .execute(&ChapterStep::get_info_by_id(chapter_id))
        .await?;

    check_team_admin_by_comic(proxy, user_id, &chapter_info.comic_id).await
}

/// Verify the caller is assigned as a reviewer on this chapter.
async fn check_reviewer<P>(proxy: &mut P, user_id: &str, chapter_id: &str) -> RootResult<()>
where
    P: for<'a> ProxyExecute<GetByChapterUserId<'a>, Error = RootError>,
{
    let assignment_info = proxy
        .execute(&AssignmentStep::get_by_chapter_user_id(chapter_id, user_id))
        .await?;

    let Some(assignment_info) = assignment_info else {
        return Err(chapter_reviewer_error());
    };
    if !assignment_info.roles.has_any_role(&[RoleField::REVIEWER]) {
        return Err(chapter_reviewer_error());
    }

    accept(())
}

/// Verify the caller is permitted to perform the given workflow transition
/// on the chapter. Reviewers bypass per-stage checks and are allowed any
/// transition. Non-reviewer assignments are validated against a whitelist:
///
/// | Stage | Event | Required role |
/// |---|---|---|
/// | `RawProvide` | `Advance` | `RAW_PROVIDER` |
/// | `Translate` | `Advance` | `TRANSLATOR` |
/// | `Translate` | `Revert` | `PROOFREADER` |
/// | `Proofread` | `Advance`/`Revert` | `PROOFREADER` |
/// | `TypesetRedraw` | `Advance`/`Revert` | `TYPESETTER` or `REDRAWER` |
/// | `Publish` | `Advance` | `PUBLISHER` |
async fn check_workflow_role<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
    stage: WorkflowStage,
    event: WorkflowEvent,
) -> RootResult<()>
where
    P: for<'a> ProxyExecute<GetByChapterUserId<'a>, Error = RootError>,
{
    let assignment_info = proxy
        .execute(&AssignmentStep::get_by_chapter_user_id(chapter_id, user_id))
        .await?;

    let Some(assignment_info) = assignment_info else {
        return Err(chapter_workflow_role_error());
    };

    let role_mask = assignment_info.roles;
    if role_mask.has_any_role(&[RoleField::REVIEWER]) {
        return accept(());
    }

    let allowed = match (stage, event) {
        (WorkflowStage::RawProvide, WorkflowEvent::Advance) => {
            role_mask.has_any_role(&[RoleField::RAW_PROVIDER])
        }
        (WorkflowStage::Translate, WorkflowEvent::Advance) => {
            role_mask.has_any_role(&[RoleField::TRANSLATOR])
        }
        (WorkflowStage::Translate, WorkflowEvent::Revert) => {
            role_mask.has_any_role(&[RoleField::PROOFREADER])
        }
        (WorkflowStage::Proofread, WorkflowEvent::Advance | WorkflowEvent::Revert) => {
            role_mask.has_any_role(&[RoleField::PROOFREADER])
        }
        (WorkflowStage::TypesetRedraw, WorkflowEvent::Advance | WorkflowEvent::Revert) => {
            role_mask.has_any_role(&[RoleField::TYPESETTER, RoleField::REDRAWER])
        }
        (WorkflowStage::Publish, WorkflowEvent::Advance) => {
            role_mask.has_any_role(&[RoleField::PUBLISHER])
        }
        _ => false,
    };

    if !allowed {
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
    role_mask: RoleMask,
) -> RootResult<()>
where
    P: for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
        + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
        + for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
{
    if role_mask.has_any_role(&[RoleField::ADMIN]) {
        return Err(chapter_role_not_assignable_args_error());
    }

    let team_id = resolve_team_id_from_comic(proxy, &chapter_info.comic_id).await?;
    let member_info = proxy
        .execute(&MemberStep::find_by_user_team_id(user_id, &team_id))
        .await?;

    let Some(member_info) = member_info else {
        return Err(RootError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-team-member-required"),
        });
    };
    if !member_info.roles.contains_mask(role_mask) {
        return Err(RootError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-chapter-role-not-assignable"),
        });
    }

    accept(())
}

/// Resolve the owning team identifier by fetching a comic and its parent workset.
async fn resolve_team_id_from_comic<P>(proxy: &mut P, comic_id: &str) -> RootResult<String>
where
    P: for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
        + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>,
{
    let comic_info = proxy.execute(&ComicStep::get_info_by_id(comic_id)).await?;

    let workset_info = proxy
        .execute(&WorksetStep::get_info_by_id(&comic_info.workset_id))
        .await?;

    accept(workset_info.team_id)
}

/// Construct a "chapter reviewer required" permission error.
fn chapter_reviewer_error() -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-chapter-reviewer-required"),
    }
}

/// Construct a "workflow role required for this transition" permission error.
fn chapter_workflow_role_error() -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-chapter-workflow-role-required"),
    }
}

/// Construct an "admin role not assignable through join" args error.
fn chapter_role_not_assignable_args_error() -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-chapter-role-not-assignable"),
    }
}
