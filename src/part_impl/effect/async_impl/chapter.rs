//! Chapter event handlers for async side effects.

use std::borrow::Cow;
use std::collections::HashMap;

use fluent_templates::fluent_bundle::FluentValue;
use poprako_orchestra::{Context, OperRun as _};
use tracing::instrument;

use poprako_util::i18n::{trl, trl_kv};

use crate::complex::system_mail::SystemMailComplex;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::write::system_mail::SystemMailEntry;
use crate::part::effect::event::chapter::{
    ChapterPublishedEvent, ChapterWorkflowCompletedEvent,
};
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::oper::assignment::ListAssignmentInfos;
use crate::part::repo::oper::chapter::GetChapterInfo;
use crate::part::repo::oper::system_mail::SendSystemMails;
use crate::part::repo::system_mail::SystemMailRepo;
use crate::value::chapter::{ChapterInclOpt, Stage};
use crate::value::role::RoleField;

// Default include options for loading chapter data with its comic, workset, and team relations.
const CHAPTER_INCL_OPT: &[ChapterInclOpt] = &[ChapterInclOpt::ComicWorksetTeam];

// Maximum number of characters for truncated comic titles in notification messages.
const TITLE_LIMIT: usize = 15;

/// Notifies next-phase assignees after one workflow stage completes.
#[instrument(level = "info", skip_all)]
pub async fn notify_next_phase<C, R>(
    repo: &R,
    event: &ChapterWorkflowCompletedEvent,
) where
    C: Context,
    R: AssignmentRepo<C> + ChapterRepo<C> + SystemMailRepo,
{
    let Some((receiver_role, workflow_label)) =
        next_phase_config(event.completed_stage)
    else {
        return;
    };

    let Some(chapter_info) = load_chapter(repo, &event.chapter_id).await else {
        return;
    };

    let system_mail_entries = build_assignment_mails(
        repo,
        &chapter_info,
        receiver_role,
        workflow_label,
    )
    .await;

    send_batch(repo, &event.chapter_id, system_mail_entries).await;
}

/// Notifies reviewer assignees after workflow progress, except typesetting completion.
#[instrument(level = "info", skip_all)]
pub async fn notify_reviewers_on_progress<C, R>(
    repo: &R,
    event: ChapterWorkflowCompletedEvent,
) where
    C: Context,
    R: AssignmentRepo<C> + ChapterRepo<C> + SystemMailRepo,
{
    let Some(workflow_label) = reviewer_progress_label(event.completed_stage)
    else {
        return;
    };

    notify_reviewers(repo, &event.chapter_id, workflow_label).await;
}

/// Notifies reviewer assignees when a chapter is published.
#[instrument(level = "info", skip_all)]
pub async fn notify_reviewers_on_publish<C, R>(
    repo: &R,
    event: ChapterPublishedEvent,
) where
    C: Context,
    R: AssignmentRepo<C> + ChapterRepo<C> + SystemMailRepo,
{
    notify_reviewers(repo, &event.chapter_id, trl("mail-workflow-publish"))
        .await;
}

// Returns the next-phase role and workflow label for a completed stage.
// `Publish` does not generate a next-phase notification.
fn next_phase_config(stage: Stage) -> Option<(RoleField, String)> {
    //
    match stage {
        //
        // Internal implementation detail.
        Stage::RawProvide => {
            Some((RoleField::TRANSLATOR, trl("mail-workflow-upload")))
        }

        Stage::Translate => {
            Some((RoleField::PROOFREADER, trl("mail-workflow-translate")))
        }

        Stage::Proofread => {
            Some((RoleField::TYPESETTER, trl("mail-workflow-proofread")))
        }

        Stage::TypesetRedraw => {
            Some((RoleField::REVIEWER, trl("mail-workflow-typeset")))
        }

        Stage::Review => {
            Some((RoleField::PUBLISHER, trl("mail-workflow-review")))
        }

        Stage::Publish => None,
    }
}

/// Loads a chapter by ID with default include options, returning `None` on lookup failure.
#[instrument(level = "info", skip_all)]
// Loads the chapter and resolves include data used by notification templates.
async fn load_chapter<C, R>(repo: &R, chapter_id: &str) -> Option<ChapterInfo>
where
    C: Context,
    R: ChapterRepo<C>,
{
    let chapter_info = GetChapterInfo {
        id: chapter_id,
        incls: CHAPTER_INCL_OPT,
    }
    .run_on(repo)
    .await;

    let Ok(chapter_info) = chapter_info else {
        //
        // Internal implementation detail.
        tracing::warn!(
            chapter_id = %chapter_id,
            "[AsyncEffectDevelop::load_chapter] failed to look up chapter for notification",
        );

        return None;
    };

    Some(chapter_info)
}

/// Builds a list of system mail forms for all assignments in a chapter matching a role.
#[instrument(level = "info", skip_all)]
// Fetches assignments and renders localized title, workflow, and assignee fields.
async fn build_assignment_mails<C, R>(
    repo: &R,
    chapter_info: &ChapterInfo,
    receiver_role: RoleField,
    workflow_label: String,
) -> Vec<SystemMailEntry>
where
    C: Context,
    R: AssignmentRepo<C>,
{
    let assignment_infos = ListAssignmentInfos::Chapter {
        chapter_id: &chapter_info.id,
        role: Some(receiver_role),
        incls: &[],
    }
    .run_on(repo)
    .await;

    let Ok(assignment_infos) = assignment_infos else {
        //
        // Internal implementation detail.
        tracing::warn!(
            chapter_id = %chapter_info.id,
            "[AsyncEffectDevelop::build_assignment_mails] failed to list chapter assignments",
        );

        return Vec::new();
    };

    let Some(args) = chapter_mail_args(chapter_info, workflow_label) else {
        //
        // Internal implementation detail.
        tracing::warn!(
            chapter_id = %chapter_info.id,
            "[AsyncEffectDevelop::build_assignment_mails] missing chapter include chain",
        );

        return Vec::new();
    };

    let (title, content) = (
        trl_kv("mail-chapter-progress-title", &args),
        trl_kv("mail-chapter-progress-body", &args),
    );

    assignment_infos
        .into_iter()
        .map(|assignment_info| SystemMailEntry {
            id: SystemMailComplex::gen_id(),
            receiver_id: assignment_info.user_id,
            title: title.clone(),
            content: content.clone(),
        })
        .collect()
}

/// Sends a batch of system mail forms, logging a warning on failure.
#[instrument(level = "info", skip_all)]
// Submits prepared mails and logs the failure path for observability.
async fn send_batch<R>(
    repo: &R,
    chapter_id: &str,
    system_mail_entries: Vec<SystemMailEntry>,
) where
    R: SystemMailRepo,
{
    if system_mail_entries.is_empty() {
        return;
    }

    if (SendSystemMails {
        entries: &system_mail_entries,
    })
    .run_on(repo)
    .await
    .is_err()
    {
        tracing::warn!(
            chapter_id = %chapter_id,
            "[AsyncEffectDevelop::send_batch] failed to send chapter notification mails",
        );
    }
}

// Returns the reviewer workflow label for a completed stage, skipping typesetting.
// Returns `None` when reviewers are not expected.
fn reviewer_progress_label(stage: Stage) -> Option<String> {
    //
    match stage {
        //
        // Internal implementation detail.
        Stage::RawProvide => Some(trl("mail-workflow-upload")),

        Stage::Translate => Some(trl("mail-workflow-translate")),

        Stage::Proofread => Some(trl("mail-workflow-proofread")),

        Stage::TypesetRedraw => None,

        Stage::Review => Some(trl("mail-workflow-review")),

        Stage::Publish => None,
    }
}

/// Notifies all reviewer assignees of a chapter about a workflow event.
#[instrument(level = "info", skip_all)]
// Loads current assignees, builds their mail payload, then dispatches notifications.
async fn notify_reviewers<C, R>(
    repo: &R,
    chapter_id: &str,
    workflow_label: String,
) where
    C: Context,
    R: AssignmentRepo<C> + ChapterRepo<C> + SystemMailRepo,
{
    let Some(chapter_info) = load_chapter(repo, chapter_id).await else {
        return;
    };

    let system_mail_entries = build_assignment_mails(
        repo,
        &chapter_info,
        RoleField::REVIEWER,
        workflow_label,
    )
    .await;

    send_batch(repo, chapter_id, system_mail_entries).await;
}

// Builds the i18n template arguments for a chapter progress notification mail.
// Returns `None` when any required include (comic/workset/team) is missing.
fn chapter_mail_args(
    chapter_info: &ChapterInfo,
    workflow_label: String,
) -> Option<HashMap<Cow<'static, str>, FluentValue<'static>>> {
    //
    // Internal implementation detail.
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
        FluentValue::String(Cow::Owned((comic_info.index + 1).to_string())),
    );

    args.insert(Cow::Borrowed("comic_title"), FluentValue::from(short_title));

    args.insert(
        Cow::Borrowed("chapter_index"),
        FluentValue::String(Cow::Owned((chapter_info.index + 1).to_string())),
    );

    args.insert(Cow::Borrowed("workflow"), FluentValue::from(workflow_label));

    Some(args)
}

// Truncates a title to a maximum number of characters, appending ellipsis if truncated.
fn truncate_title(title: &str, max_chars: usize) -> String {
    //
    // Internal implementation detail.
    let mut chars = title.chars();

    let short_title = chars.by_ref().take(max_chars).collect::<String>();

    match chars.next() {
        //
        // Internal implementation detail.
        Some(_) => format!("{}...", short_title),

        None => short_title,
    }
}
