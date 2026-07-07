//! Chapter event handlers for async side effects.

use std::borrow::Cow;
use std::collections::HashMap;

use fluent_templates::fluent_bundle::FluentValue;

use poprako_util::i18n::{trl, trl_kv};

use crate::complex::system_mail::SystemMailComplex;
use crate::model::chapter::ChapterInfo;
use crate::model::system_mail::SystemMailForm;
use crate::part::effect::event::chapter::{
    ChapterPublishedPayload, ChapterWorkflowCompletedPayload,
};
use crate::part::repo::assignment::{AssignmentRepo, AssignmentRepoTransactional};
use crate::part::repo::chapter::{ChapterRepo, ChapterRepoTransactional};
use crate::part::repo::step::assignment::AssignmentStep;
use crate::part::repo::step::chapter::ChapterStep;
use crate::part::repo::step::system_mail::SystemMailStep;
use crate::part::repo::system_mail::{SystemMailRepo, SystemMailRepoTransactional};
use crate::part::shared::execute::Execute;
use crate::util::DeriveTransactional;
use crate::value::chapter::{ChapterInclOpt, Stage};
use crate::value::role::RoleField;

const CHAPTER_INCL_OPT: &[ChapterInclOpt] = &[ChapterInclOpt::ComicWorksetTeam];
const TITLE_LIMIT: usize = 15;

/// Notifies next-phase assignees after one workflow stage completes.
pub async fn notify_next_phase<C, R>(repo: &R, payload: &ChapterWorkflowCompletedPayload)
where
    R: AssignmentRepo<C> + ChapterRepo<C> + SystemMailRepo<C>,
    <R as DeriveTransactional>::Transactional: AssignmentRepoTransactional<C>
        + ChapterRepoTransactional<C>
        + SystemMailRepoTransactional<C>,
{
    let Some((receiver_role, workflow_label)) = next_phase_config(payload.completed_stage) else {
        return;
    };

    let Some(chapter_info) = load_chapter(repo, &payload.chapter_id).await else {
        return;
    };

    let system_mail_forms =
        build_assignment_mails(repo, &chapter_info, receiver_role, workflow_label).await;

    send_batch(repo, &payload.chapter_id, system_mail_forms).await;
}

/// Notifies reviewer assignees after workflow progress, except typesetting completion.
pub async fn notify_reviewers_on_progress<C, R>(repo: &R, payload: ChapterWorkflowCompletedPayload)
where
    R: AssignmentRepo<C> + ChapterRepo<C> + SystemMailRepo<C>,
    <R as DeriveTransactional>::Transactional: AssignmentRepoTransactional<C>
        + ChapterRepoTransactional<C>
        + SystemMailRepoTransactional<C>,
{
    let Some(workflow_label) = reviewer_progress_label(payload.completed_stage) else {
        return;
    };

    notify_reviewers(repo, &payload.chapter_id, workflow_label).await;
}

/// Notifies reviewer assignees when a chapter is published.
pub async fn notify_reviewers_on_publish<C, R>(repo: &R, payload: ChapterPublishedPayload)
where
    R: AssignmentRepo<C> + ChapterRepo<C> + SystemMailRepo<C>,
    <R as DeriveTransactional>::Transactional: AssignmentRepoTransactional<C>
        + ChapterRepoTransactional<C>
        + SystemMailRepoTransactional<C>,
{
    notify_reviewers(repo, &payload.chapter_id, trl("mail-workflow-publish")).await;
}

async fn notify_reviewers<C, R>(repo: &R, chapter_id: &str, workflow_label: String)
where
    R: AssignmentRepo<C> + ChapterRepo<C> + SystemMailRepo<C>,
    <R as DeriveTransactional>::Transactional: AssignmentRepoTransactional<C>
        + ChapterRepoTransactional<C>
        + SystemMailRepoTransactional<C>,
{
    let Some(chapter_info) = load_chapter(repo, chapter_id).await else {
        return;
    };

    let system_mail_forms =
        build_assignment_mails(repo, &chapter_info, RoleField::REVIEWER, workflow_label).await;

    send_batch(repo, chapter_id, system_mail_forms).await;
}

async fn load_chapter<C, R>(repo: &R, chapter_id: &str) -> Option<ChapterInfo>
where
    R: ChapterRepo<C>,
    <R as DeriveTransactional>::Transactional: ChapterRepoTransactional<C>,
{
    let chapter_info = Execute::execute(
        repo,
        &ChapterStep::get_info_by_id(chapter_id, CHAPTER_INCL_OPT),
    )
    .await;

    let Ok(chapter_info) = chapter_info else {
        tracing::warn!(
            chapter_id = %chapter_id,
            "[AsyncEffectDevelop::load_chapter] failed to look up chapter for notification",
        );

        return None;
    };

    Some(chapter_info)
}

async fn build_assignment_mails<C, R>(
    repo: &R,
    chapter_info: &ChapterInfo,
    receiver_role: RoleField,
    workflow_label: String,
) -> Vec<SystemMailForm>
where
    R: AssignmentRepo<C>,
    <R as DeriveTransactional>::Transactional: AssignmentRepoTransactional<C>,
{
    let assignment_infos = Execute::execute(
        repo,
        &AssignmentStep::list_all_infos_by_chapter(
            &chapter_info.id,
            Some(receiver_role),
            &[],
        ),
    )
    .await;

    let Ok(assignment_infos) = assignment_infos else {
        tracing::warn!(
            chapter_id = %chapter_info.id,
            "[AsyncEffectDevelop::build_assignment_mails] failed to list chapter assignments",
        );

        return Vec::new();
    };

    let Some(args) = chapter_mail_args(chapter_info, workflow_label) else {
        tracing::warn!(
            chapter_id = %chapter_info.id,
            "[AsyncEffectDevelop::build_assignment_mails] missing chapter include chain",
        );

        return Vec::new();
    };

    let title = trl_kv("mail-chapter-progress-title", &args);
    let content = trl_kv("mail-chapter-progress-body", &args);

    assignment_infos
        .into_iter()
        .map(|assignment_info| SystemMailForm {
            id: SystemMailComplex::gen_id(),
            receiver_id: assignment_info.user_id,
            title: title.clone(),
            content: content.clone(),
        })
        .collect()
}

fn chapter_mail_args(
    chapter_info: &ChapterInfo,
    workflow_label: String,
) -> Option<HashMap<Cow<'static, str>, FluentValue<'static>>> {
    let comic_info = chapter_info.comic.as_ref()?;

    let workset_info = comic_info.workset.as_ref()?;

    let team_info = comic_info.team.as_ref()?;

    let short_title = truncate_title(&comic_info.title, TITLE_LIMIT);

    let mut args = HashMap::new();
    args.insert(
        Cow::Borrowed("team_name"),
        FluentValue::from(team_info.name.clone()),
    );
    args.insert(
        Cow::Borrowed("workset_name"),
        FluentValue::from(workset_info.name.clone()),
    );
    args.insert(
        Cow::Borrowed("comic_index"),
        FluentValue::from(i64::from(comic_info.index + 1)),
    );
    args.insert(Cow::Borrowed("comic_title"), FluentValue::from(short_title));
    args.insert(
        Cow::Borrowed("chapter_index"),
        FluentValue::from(i64::from(chapter_info.index + 1)),
    );
    args.insert(Cow::Borrowed("workflow"), FluentValue::from(workflow_label));

    Some(args)
}

async fn send_batch<C, R>(repo: &R, chapter_id: &str, system_mail_forms: Vec<SystemMailForm>)
where
    R: SystemMailRepo<C>,
    <R as DeriveTransactional>::Transactional: SystemMailRepoTransactional<C>,
{
    if system_mail_forms.is_empty() {
        return;
    }

    let result = Execute::execute(repo, &SystemMailStep::send_batch(&system_mail_forms)).await;

    if result.is_err() {
        tracing::warn!(
            chapter_id = %chapter_id,
            "[AsyncEffectDevelop::send_batch] failed to send chapter notification mails",
        );
    }
}

fn next_phase_config(stage: Stage) -> Option<(RoleField, String)> {
    match stage {
        Stage::RawProvide => Some((RoleField::TRANSLATOR, trl("mail-workflow-upload"))),
        Stage::Translate => Some((RoleField::PROOFREADER, trl("mail-workflow-translate"))),
        Stage::Proofread => Some((RoleField::TYPESETTER, trl("mail-workflow-proofread"))),
        Stage::TypesetRedraw => Some((RoleField::REVIEWER, trl("mail-workflow-typeset"))),
        Stage::Review => Some((RoleField::PUBLISHER, trl("mail-workflow-review"))),
        Stage::Publish => None,
    }
}

fn reviewer_progress_label(stage: Stage) -> Option<String> {
    match stage {
        Stage::RawProvide => Some(trl("mail-workflow-upload")),
        Stage::Translate => Some(trl("mail-workflow-translate")),
        Stage::Proofread => Some(trl("mail-workflow-proofread")),
        Stage::TypesetRedraw => None,
        Stage::Review => Some(trl("mail-workflow-review")),
        Stage::Publish => None,
    }
}

fn truncate_title(title: &str, max_chars: usize) -> String {
    let mut chars = title.chars();

    let short_title: String = chars.by_ref().take(max_chars).collect();

    match chars.next() {
        Some(_) => format!("{}...", short_title),
        None => short_title,
    }
}
