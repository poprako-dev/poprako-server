//! Chapter event handlers for async side effects.

use std::borrow::Cow;
use std::collections::HashMap;

use fluent_templates::fluent_bundle::FluentValue;

use poprako_util::i18n::{trl, trl_kv};

use crate::complex::system_mail::SystemMailComplex;
use crate::model::chapter::ChapterInfo;
use crate::model::system_mail::SystemMailEntry;
use crate::part::effect::event::chapter::{
    ChapterPublishedPayload, ChapterWorkflowCompletedPayload,
};
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::oper::assignment::ListAssignmentInfos;
use crate::part::repo::oper::chapter::GetChapterInfo;
use crate::part::repo::oper::system_mail::SendSystemMails;
use crate::part::repo::system_mail::SystemMailRepo;
use crate::value::chapter::{ChapterInclOpt, Stage};
use crate::value::role::RoleField;

/// Default include options for loading chapter data with its comic, workset, and team relations.
const CHAPTER_INCL_OPT: &[ChapterInclOpt] = &[ChapterInclOpt::ComicWorksetTeam];

/// Maximum number of characters for truncated comic titles in notification messages.
const TITLE_LIMIT: usize = 15;

/// Notifies next-phase assignees after one workflow stage completes.
pub async fn notify_next_phase<C, R>(
    repo: &R,
    payload: &ChapterWorkflowCompletedPayload,
) where
    R: AssignmentRepo<C> + ChapterRepo<C> + SystemMailRepo<C>,
{
    let Some((receiver_role, workflow_label)) =
        next_phase_config(payload.completed_stage)
    else {
        return;
    };

    let Some(chapter_info) = load_chapter(repo, &payload.chapter_id).await
    else {
        return;
    };

    let system_mail_entries = build_assignment_mails(
        repo,
        &chapter_info,
        receiver_role,
        workflow_label,
    )
    .await;

    send_batch(repo, &payload.chapter_id, system_mail_entries).await;
}

/// Notifies reviewer assignees after workflow progress, except typesetting completion.
pub async fn notify_reviewers_on_progress<C, R>(
    repo: &R,
    payload: ChapterWorkflowCompletedPayload,
) where
    R: AssignmentRepo<C> + ChapterRepo<C> + SystemMailRepo<C>,
{
    let Some(workflow_label) = reviewer_progress_label(payload.completed_stage)
    else {
        return;
    };

    notify_reviewers(repo, &payload.chapter_id, workflow_label).await;
}

/// Notifies reviewer assignees when a chapter is published.
pub async fn notify_reviewers_on_publish<C, R>(
    repo: &R,
    payload: ChapterPublishedPayload,
) where
    R: AssignmentRepo<C> + ChapterRepo<C> + SystemMailRepo<C>,
{
    notify_reviewers(repo, &payload.chapter_id, trl("mail-workflow-publish"))
        .await;
}

/// Notifies all reviewer assignees of a chapter about a workflow event.
async fn notify_reviewers<C, R>(
    repo: &R,
    chapter_id: &str,
    workflow_label: String,
) where
    R: AssignmentRepo<C> + ChapterRepo<C> + SystemMailRepo<C>,
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

/// Loads a chapter by ID with default include options, returning `None` on lookup failure.
async fn load_chapter<C, R>(repo: &R, chapter_id: &str) -> Option<ChapterInfo>
where
    R: ChapterRepo<C>,
{
    let chapter_info = repo
        .run(&GetChapterInfo {
            id: chapter_id,
            incls: CHAPTER_INCL_OPT,
        })
        .await;

    let Ok(chapter_info) = chapter_info else {
        //
        tracing::warn!(
            chapter_id = %chapter_id,
            "[AsyncEffectDevelop::load_chapter] failed to look up chapter for notification",
        );

        return None;
    };

    Some(chapter_info)
}

/// Builds a list of system mail forms for all assignments in a chapter matching a role.
async fn build_assignment_mails<C, R>(
    repo: &R,
    chapter_info: &ChapterInfo,
    receiver_role: RoleField,
    workflow_label: String,
) -> Vec<SystemMailEntry>
where
    R: AssignmentRepo<C>,
{
    let list_assignment_infos = ListAssignmentInfos::Chapter {
        chapter_id: &chapter_info.id,
        role: Some(receiver_role),
        incls: &[],
    };

    let assignment_infos = repo.run(&list_assignment_infos).await;

    let Ok(assignment_infos) = assignment_infos else {
        //
        tracing::warn!(
            chapter_id = %chapter_info.id,
            "[AsyncEffectDevelop::build_assignment_mails] failed to list chapter assignments",
        );

        return Vec::new();
    };

    let Some(args) = chapter_mail_args(chapter_info, workflow_label) else {
        //
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
        .map(|assignment_info| SystemMailEntry {
            id: SystemMailComplex::gen_id(),
            receiver_id: assignment_info.user_id,
            title: title.clone(),
            content: content.clone(),
        })
        .collect()
}

/// Builds the i18n template arguments for a chapter progress notification mail.
fn chapter_mail_args(
    chapter_info: &ChapterInfo,
    workflow_label: String,
) -> Option<HashMap<Cow<'static, str>, FluentValue<'static>>> {
    //
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

/// Sends a batch of system mail forms, logging a warning on failure.
async fn send_batch<C, R>(
    repo: &R,
    chapter_id: &str,
    system_mail_entries: Vec<SystemMailEntry>,
) where
    R: SystemMailRepo<C>,
{
    if system_mail_entries.is_empty() {
        return;
    }

    if repo
        .run(&SendSystemMails {
            entries: &system_mail_entries,
        })
        .await
        .is_err()
    {
        tracing::warn!(
            chapter_id = %chapter_id,
            "[AsyncEffectDevelop::send_batch] failed to send chapter notification mails",
        );
    }
}

/// Returns the next-phase role and workflow label for a completed stage.
fn next_phase_config(stage: Stage) -> Option<(RoleField, String)> {
    match stage {
        //
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

/// Returns the reviewer workflow label for a completed stage, skipping typesetting.
fn reviewer_progress_label(stage: Stage) -> Option<String> {
    match stage {
        //
        Stage::RawProvide => Some(trl("mail-workflow-upload")),

        Stage::Translate => Some(trl("mail-workflow-translate")),

        Stage::Proofread => Some(trl("mail-workflow-proofread")),

        Stage::TypesetRedraw => None,

        Stage::Review => Some(trl("mail-workflow-review")),

        Stage::Publish => None,
    }
}

/// Truncates a title to a maximum number of characters, appending ellipsis if truncated.
fn truncate_title(title: &str, max_chars: usize) -> String {
    //
    let mut chars = title.chars();

    let short_title: String = chars.by_ref().take(max_chars).collect();

    match chars.next() {
        //
        Some(_) => format!("{}...", short_title),

        None => short_title,
    }
}
