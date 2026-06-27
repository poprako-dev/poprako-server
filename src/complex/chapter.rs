//! Complex-domain operations for chapter entities.

use time::OffsetDateTime;

use poprako_util::i18n::trl;

use crate::complex::image::ImageComplex;
use crate::complex::util::{check_user_is_team_admin, check_user_is_team_member};
use crate::model::chapter::{ChapterInfo, ChapterInfoUpdate, ChapterStageUpdate};
use crate::model::role::{RoleBit, RoleMask};
use crate::part::prom::intention::{IMAGE_TOPIC, ImageIntention};
use crate::part::prom::{Payload, PromStep, PromTransactional};
use crate::part::repo::assignment::AssignmentRepoTransactional;
use crate::part::repo::chapter::ChapterRepoTransactional;
use crate::part::repo::comic::ComicRepoTransactional;
use crate::part::repo::page::PageRepoTransactional;
use crate::part::repo::proxy::ProxyExecute;
use crate::part::repo::step::assignment::{AssignmentStep, GetByChapterUserId};
use crate::part::repo::step::chapter::{ChapterStep, GetInfoById as ChapterGetInfoById};
use crate::part::repo::step::comic::{ComicStep, GetInfoById as ComicGetInfoById};
use crate::part::repo::step::member::{FindByUserTeamId, MemberStep};
use crate::part::repo::step::page::PageStep;
use crate::part::repo::step::workset::{GetInfoById as WorksetGetInfoById, WorksetStep};
use crate::result::{ExpectedVariant, RootError, RootResult, accept};
use crate::util::next_snowflake_id;
use crate::value::chapter::{StagePhase, WorkflowEvent, WorkflowStage, try_modify_stage};

/// Domain operations for chapter entities.
pub struct ChapterComplex;

impl ChapterComplex {
    pub fn gen_id() -> String {
        next_snowflake_id()
    }

    fn default_subtitle(index: i32) -> String {
        format!("第 {} 话", index + 1)
    }

    pub fn subtitle_or_default(subtitle: Option<String>, index: i32) -> String {
        subtitle
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| Self::default_subtitle(index))
    }

    pub fn build_stage_update(
        chapter_info: &ChapterInfo,
        stage: WorkflowStage,
        event: WorkflowEvent,
    ) -> RootResult<ChapterStageUpdate> {
        let current_phase = phase(chapter_info, stage);
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

    pub async fn clear_page_images<C, R, P>(
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
        enqueue_page_image_deletes(repo, prom, context, chapter_id).await?;

        repo.advance(context, &PageStep::clear_images_by_chapter(chapter_id))
            .await?;

        accept(())
    }

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
            + AssignmentRepoTransactional<C>
            + PageRepoTransactional<C>
            + Send
            + Sync,
        P: PromTransactional<C> + Send + Sync,
    {
        let chapter_info = repo
            .advance(context, &ChapterStep::get_info_excluded(id))
            .await?;

        enqueue_page_image_deletes(repo, prom, context, &chapter_info.id).await?;

        repo.advance(context, &PageStep::delete_by_chapter(&chapter_info.id))
            .await?;

        repo.advance(
            context,
            &AssignmentStep::delete_by_chapter(&chapter_info.id),
        )
        .await?;

        repo.advance(context, &ChapterStep::delete(&chapter_info.id))
            .await?;

        repin_latest_chapter(repo, context, &chapter_info).await?;

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

fn phase(chapter_info: &ChapterInfo, stage: WorkflowStage) -> StagePhase {
    match stage {
        WorkflowStage::RawProvide => chapter_info.raw_provide_phase,
        WorkflowStage::Translate => chapter_info.translate_phase,
        WorkflowStage::Proofread => chapter_info.proofread_phase,
        WorkflowStage::TypesetRedraw => chapter_info.typeset_redraw_phase,
        WorkflowStage::Review => chapter_info.review_phase,
        WorkflowStage::Publish => chapter_info.publish_phase,
    }
}

async fn repin_latest_chapter<C, R>(
    repo: &R,
    context: &mut C,
    chapter_info: &ChapterInfo,
) -> RootResult<()>
where
    C: Send,
    R: ChapterRepoTransactional<C> + Send + Sync,
{
    if !chapter_info.is_pinned {
        return accept(());
    }

    let remaining_chapter_infos = repo
        .advance(
            context,
            &ChapterStep::list_by_comic_id_excluded(&chapter_info.comic_id, 0, 1),
        )
        .await?;

    let Some(remaining_chapter_info) = remaining_chapter_infos.first() else {
        return accept(());
    };

    let chapter_info_update = ChapterInfoUpdate {
        id: remaining_chapter_info.id.clone(),
        subtitle: None,
        is_pinned: Some(true),
    };

    repo.advance(context, &ChapterStep::update_info(&chapter_info_update))
        .await?;

    accept(())
}

async fn enqueue_page_image_deletes<C, R, P>(
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
        .advance(context, &PageStep::list_by_chapter(chapter_id))
        .await?;

    for page_info in page_infos {
        if !page_info.image_uploaded {
            continue;
        }

        let Some(image_key) = page_info.image_key else {
            continue;
        };

        if image_key.is_empty() {
            continue;
        }

        let delete_id = ImageComplex::gen_delete_id();

        let now = OffsetDateTime::now_utc();

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

    accept(())
}

/// Permission-gate operations for chapter entities.
pub struct ChapterPermComplex;

impl ChapterPermComplex {
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

    pub async fn can_user_create<P>(proxy: &mut P, user_id: &str, comic_id: &str) -> RootResult<()>
    where
        P: for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RootError>
            + for<'a> ProxyExecute<FindByUserTeamId<'a>, Error = RootError>,
    {
        check_team_admin_by_comic(proxy, user_id, comic_id).await
    }

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
    if !assignment_info.role_mask.has_any_role(&[RoleBit::REVIEWER]) {
        return Err(chapter_reviewer_error());
    }

    accept(())
}

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

    let role_mask = assignment_info.role_mask;
    if role_mask.has_any_role(&[RoleBit::REVIEWER]) {
        return accept(());
    }

    let allowed = match (stage, event) {
        (WorkflowStage::RawProvide, WorkflowEvent::Advance) => {
            role_mask.has_any_role(&[RoleBit::RAW_PROVIDER])
        }
        (WorkflowStage::Translate, WorkflowEvent::Advance) => {
            role_mask.has_any_role(&[RoleBit::TRANSLATOR])
        }
        (WorkflowStage::Translate, WorkflowEvent::Revert) => {
            role_mask.has_any_role(&[RoleBit::PROOFREADER])
        }
        (WorkflowStage::Proofread, WorkflowEvent::Advance | WorkflowEvent::Revert) => {
            role_mask.has_any_role(&[RoleBit::PROOFREADER])
        }
        (WorkflowStage::TypesetRedraw, WorkflowEvent::Advance | WorkflowEvent::Revert) => {
            role_mask.has_any_role(&[RoleBit::TYPESETTER, RoleBit::REDRAWER])
        }
        (WorkflowStage::Publish, WorkflowEvent::Advance) => {
            role_mask.has_any_role(&[RoleBit::PUBLISHER])
        }
        _ => false,
    };

    if !allowed {
        return Err(chapter_workflow_role_error());
    }

    accept(())
}

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
    if role_mask.has_any_role(&[RoleBit::ADMIN]) {
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
    if !member_info.role_mask.contains_mask(role_mask) {
        return Err(RootError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-chapter-role-not-assignable"),
        });
    }

    accept(())
}

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

fn chapter_reviewer_error() -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-chapter-reviewer-required"),
    }
}

fn chapter_workflow_role_error() -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-chapter-workflow-role-required"),
    }
}

fn chapter_role_not_assignable_args_error() -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-chapter-role-not-assignable"),
    }
}
