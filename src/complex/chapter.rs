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
use time::OffsetDateTime;

use poprako_util::i18n::{trl, trl_kv};

use crate::complex::image::ImageComplex;
use crate::complex::util::{check_user_is_team_admin, check_user_is_team_member};
use crate::model::chapter::{ChapterInfo, ChapterInfoUpdate, ChapterStageUpdate};
use crate::part::prom::intention::{IMAGE_TOPIC, ImageIntention};
use crate::part::prom::{Payload, PromStep, PromTransactional};
use crate::part::repo::chapter::ChapterRepoTransactional;
use crate::part::repo::comic::ComicRepoTransactional;
use crate::part::repo::page::PageRepoTransactional;
use crate::part::repo::step::assignment::{AssignmentStep, GetInfoByChapterIdAndUserId};
use crate::part::repo::step::chapter::{ChapterStep, GetInfoById as ChapterGetInfoById};
use crate::part::repo::step::comic::{ComicStep, GetInfoById as ComicGetInfoById};
use crate::part::repo::step::member::{FindInfoByUserIdAndTeamId, MemberStep};
use crate::part::repo::step::page::PageStep;
use crate::part::repo::step::workset::{GetInfoById as WorksetGetInfoById, WorksetStep};
use crate::part::shared::proxy::ProxyExecute;
use crate::result::{ExpectedVariant, RootError, RootResult, accept};
use crate::util::next_snowflake_id;
use crate::value::chapter::{StagePhase, WorkflowEvent, WorkflowStage, try_modify_stage};
use crate::value::role::{RoleField, RoleMask};

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

    /// Compute the next [`ChapterStageUpdate`] by applying a [`WorkflowEvent`]
    /// to the current [`WorkflowStage`] phase of a chapter.
    pub fn build_stage_update(
        chapter_info: &ChapterInfo,
        stage: WorkflowStage,
        event: WorkflowEvent,
    ) -> RootResult<ChapterStageUpdate> {
        let current_phase = get_phase(chapter_info, stage);
        let next_phase = try_modify_stage((stage, current_phase), event)?;

        let chapter_stage_update = ChapterStageUpdate {
            id: chapter_info.id.clone(),
            stages: chapter_info.stages.set_phase(stage, next_phase),
        };

        accept(chapter_stage_update)
    }

    /// Appends page image deletes inside an existing transaction context.
    pub async fn clean_uploaded_images<C, R, P>(
        repo: &R,
        prom: &P,
        context: &mut C,
        chapter_id: &str,
    ) -> RootResult<()>
    where
        C: Send,
        R: PageRepoTransactional<C> + Send + Sync,
        P: PromTransactional<C> + Send + Sync,
    {
        prom_image_deletes(repo, prom, context, chapter_id).await
    }

    /// Deletes a chapter subtree inside an existing transaction context.
    pub async fn delete_cascade<C, R, P>(
        repo: &R,
        prom: &P,
        context: &mut C,
        id: &str,
    ) -> RootResult<()>
    where
        C: Send,
        R: ChapterRepoTransactional<C>
            + ComicRepoTransactional<C>
            + PageRepoTransactional<C>
            + Send
            + Sync,
        P: PromTransactional<C> + Send + Sync,
    {
        let chapter_info = repo
            .advance(context, &ChapterStep::get_info_by_id_excluded(id))
            .await?;

        prom_image_deletes(repo, prom, context, &chapter_info.id).await?;

        repo.advance(context, &ChapterStep::delete(&chapter_info.id))
            .await?;

        if chapter_info.is_pinned {
            repin_latest_chapter(repo, context, &chapter_info.comic_id).await?;
        }

        repo.advance(
            context,
            &ComicStep::update_chapter_count(&chapter_info.comic_id, -1),
        )
        .await?;

        repo.advance(
            context,
            &ComicStep::touch_last_active(&chapter_info.comic_id),
        )
        .await?;

        accept(())
    }
}

/// Generate a human-readable default subtitle for a chapter, e.g. "Ch. 1".
fn default_subtitle(index: i32) -> String {
    let mut args = HashMap::new();
    args.insert(Cow::Borrowed("number"), FluentValue::from(index + 1));

    trl_kv("chapter-default-subtitle", &args)
}

/// Extract the current [`StagePhase`] for a given [`WorkflowStage`] from a
/// [`ChapterInfo`] record.
fn get_phase(chapter_info: &ChapterInfo, stage: WorkflowStage) -> StagePhase {
    chapter_info.stages.get_phase(stage)
}

async fn prom_image_deletes<C, R, P>(
    repo: &R,
    prom: &P,
    context: &mut C,
    chapter_id: &str,
) -> RootResult<()>
where
    C: Send,
    R: PageRepoTransactional<C> + Send + Sync,
    P: PromTransactional<C> + Send + Sync,
{
    let page_infos = repo
        .advance(context, &PageStep::list_all_infos_by_chapter_id(chapter_id))
        .await?;

    let now = OffsetDateTime::now_utc();

    for page_info in page_infos {
        if let Some(image_key) = page_info.image_key
            && page_info.image_uploaded
        {
            let delete_id = ImageComplex::gen_delete_id();

            prom.advance(
                context,
                &PromStep::append(
                    &delete_id,
                    IMAGE_TOPIC,
                    Payload::Image(ImageIntention::Delete {
                        object_key: image_key,
                    }),
                    &now,
                ),
            )
            .await?;
        }
    }

    accept(())
}

async fn repin_latest_chapter<C, R>(repo: &R, context: &mut C, comic_id: &str) -> RootResult<()>
where
    C: Send,
    R: ChapterRepoTransactional<C> + Send + Sync,
{
    let chapter_infos = repo
        .advance(
            context,
            &ChapterStep::list_all_infos_by_comic_id_excluded(comic_id),
        )
        .await?;

    let Some(chapter_info) = chapter_infos.first() else {
        return accept(());
    };

    let chapter_info_update = ChapterInfoUpdate {
        id: chapter_info.id.clone(),
        subtitle: None,
        pin: Some(true),
    };

    repo.advance(context, &ChapterStep::update_info(&chapter_info_update))
        .await?;

    repo.advance(
        context,
        &ChapterStep::unpin_others(&chapter_info.comic_id, &chapter_info.id),
    )
    .await?;

    accept(())
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
    pub async fn can_user_list_infos<P>(
        proxy: &mut P,
        user_id: &str,
        comic_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<FindInfoByUserIdAndTeamId<'a>, Error = RootError>,
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
            + for<'a> ProxyExecute<FindInfoByUserIdAndTeamId<'a>, Error = RootError>,
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
            + for<'a> ProxyExecute<FindInfoByUserIdAndTeamId<'a>, Error = RootError>,
    {
        check_team_member_by_comic(proxy, user_id, comic_id).await
    }

    /// Verify the caller is a team admin of the comic's owning workset.
    pub async fn can_user_create<P>(proxy: &mut P, user_id: &str, comic_id: &str) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<FindInfoByUserIdAndTeamId<'a>, Error = RootError>,
    {
        check_team_admin_by_comic(proxy, user_id, comic_id).await
    }

    /// Verify the caller is assigned as a chapter admin for metadata updates.
    pub async fn can_user_update_info<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<GetInfoByChapterIdAndUserId<'a>, Error = RootError>,
    {
        check_admin(proxy, user_id, chapter_id).await
    }

    /// Verify the caller has permission to apply a workflow event.
    pub async fn can_user_update_stage<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
        stage: WorkflowStage,
        event: WorkflowEvent,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<GetInfoByChapterIdAndUserId<'a>, Error = RootError>,
    {
        check_workflow_role(proxy, user_id, chapter_id, stage, event).await
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
        roles: RoleMask,
    ) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<FindInfoByUserIdAndTeamId<'a>, Error = RootError>,
    {
        check_join_role(proxy, user_id, chapter_info, roles).await
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
            + for<'a> ProxyExecute<FindInfoByUserIdAndTeamId<'a>, Error = RootError>,
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
        + for<'a> ProxyExecute<FindInfoByUserIdAndTeamId<'a>, Error = RootError>,
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
        + for<'a> ProxyExecute<FindInfoByUserIdAndTeamId<'a>, Error = RootError>,
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
        + for<'a> ProxyExecute<FindInfoByUserIdAndTeamId<'a>, Error = RootError>,
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
        + for<'a> ProxyExecute<FindInfoByUserIdAndTeamId<'a>, Error = RootError>,
{
    let chapter_info = proxy
        .execute(&ChapterStep::get_info_by_id(chapter_id))
        .await?;

    check_team_admin_by_comic(proxy, user_id, &chapter_info.comic_id).await
}

/// Verify the caller is assigned as an admin on this chapter.
async fn check_admin<P>(proxy: &mut P, user_id: &str, chapter_id: &str) -> RootResult<()>
where
    P: for<'a> ProxyExecute<GetInfoByChapterIdAndUserId<'a>, Error = RootError>,
{
    let assignment_info = proxy
        .execute(&AssignmentStep::get_info_by_chapter_id_and_user_id(
            chapter_id, user_id,
        ))
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
    P: for<'a> ProxyExecute<GetInfoByChapterIdAndUserId<'a>, Error = RootError>,
{
    let assignment_info = proxy
        .execute(&AssignmentStep::get_info_by_chapter_id_and_user_id(
            chapter_id, user_id,
        ))
        .await?;

    let Some(assignment_info) = assignment_info else {
        return Err(chapter_workflow_role_error());
    };

    let roles = assignment_info.roles;
    if roles.has_any_role(&[RoleField::REVIEWER]) {
        return accept(());
    }

    let allowed = match (stage, event) {
        (WorkflowStage::RawProvide, WorkflowEvent::Advance) => {
            roles.has_any_role(&[RoleField::RAW_PROVIDER])
        }
        (WorkflowStage::Translate, WorkflowEvent::Advance) => {
            roles.has_any_role(&[RoleField::TRANSLATOR])
        }
        (WorkflowStage::Translate, WorkflowEvent::Revert) => {
            roles.has_any_role(&[RoleField::PROOFREADER])
        }
        (WorkflowStage::Proofread, WorkflowEvent::Advance | WorkflowEvent::Revert) => {
            roles.has_any_role(&[RoleField::PROOFREADER])
        }
        (WorkflowStage::TypesetRedraw, WorkflowEvent::Advance | WorkflowEvent::Revert) => {
            roles.has_any_role(&[RoleField::TYPESETTER, RoleField::REDRAWER])
        }
        (WorkflowStage::Publish, WorkflowEvent::Advance) => {
            roles.has_any_role(&[RoleField::PUBLISHER])
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
    roles: RoleMask,
) -> RootResult<()>
where
    P: for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
        + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
        + for<'a> ProxyExecute<FindInfoByUserIdAndTeamId<'a>, Error = RootError>,
{
    if roles.has_any_role(&[RoleField::ADMIN]) {
        return Err(chapter_role_not_assignable_args_error());
    }

    let team_id = resolve_team_id_from_comic(proxy, &chapter_info.comic_id).await?;
    let member_info = proxy
        .execute(&MemberStep::find_info_by_user_id_and_team_id(
            user_id, &team_id,
        ))
        .await?;

    let Some(member_info) = member_info else {
        return Err(RootError::Expected {
            variant: ExpectedVariant::PermDeny,
            message: trl("error-team-member-required"),
        });
    };
    if !member_info.roles.contains_mask(roles) {
        return Err(RootError::Expected {
            variant: ExpectedVariant::PermDeny,
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

/// Construct a "chapter admin required" permission error.
fn chapter_admin_error() -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::PermDeny,
        message: trl("error-chapter-admin-required"),
    }
}

/// Construct a "workflow role required for this transition" permission error.
fn chapter_workflow_role_error() -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::PermDeny,
        message: trl("error-chapter-workflow-role-required"),
    }
}

/// Construct an "admin role not assignable through join" args error.
fn chapter_role_not_assignable_args_error() -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::ArgsInvalid,
        message: trl("error-chapter-role-not-assignable"),
    }
}
