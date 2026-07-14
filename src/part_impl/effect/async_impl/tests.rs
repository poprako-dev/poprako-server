// develop_dispatches_user_signup(AsyncEffectDevelop::develop)(positive): signup events should create one system mail for the invitor.
// develop_dispatches_chapter_workflow_completed(AsyncEffectDevelop::develop)(positive): workflow completion should notify next-phase and reviewer assignees.
// develop_dispatches_chapter_published(AsyncEffectDevelop::develop)(positive): chapter publication should notify reviewer assignees.
// close_is_idempotent(AsyncEffectDevelop::close)(negative): repeated close calls should return without blocking.

use super::*;

use time::OffsetDateTime;

use crate::model::assignment::AssignmentInfo;
use crate::model::chapter::ChapterInfo;
use crate::model::comic::ComicInfo;
use crate::model::team::TeamInfo;
use crate::model::workset::WorksetInfo;
use crate::part::effect::event::chapter::{
    ChapterPublishedPayload, ChapterWorkflowCompletedPayload,
};
use crate::part::effect::event::user::UserSignedUpPayload;
use crate::part_impl::repo::mock_impl::Mock;
use crate::value::chapter::{Stage, StageMask};
use crate::value::role::{RoleField, RoleMask};

fn team_info() -> TeamInfo {
    //
    let time = OffsetDateTime::now_utc();

    TeamInfo {
        id: "team-1".to_string(),
        name: "Team One".to_string(),
        description: "Team description".to_string(),
        avatar_key: None,
        avatar_uploaded: false,
        avatar_version: 0,
        created_at: time,
        updated_at: time,
    }
}

fn workset_info() -> WorksetInfo {
    //
    let time = OffsetDateTime::now_utc();

    WorksetInfo {
        id: "workset-1".to_string(),
        team_id: "team-1".to_string(),
        index: 0,
        name: "Workset One".to_string(),
        description: None,
        comic_count: 1,
        created_at: time,
        updated_at: time,
    }
}

fn comic_info() -> ComicInfo {
    //
    let time = OffsetDateTime::now_utc();

    ComicInfo {
        id: "comic-1".to_string(),
        workset_id: "workset-1".to_string(),
        index: 0,
        title: "Comic One".to_string(),
        author: "Author One".to_string(),
        description: None,
        cover_key: None,
        cover_uploaded: false,
        cover_version: 0,
        chapter_count: 1,
        creator_id: "creator-user".to_string(),
        workset: None,
        team: None,
        creator: None,
        last_active_at: time,
        created_at: time,
        updated_at: time,
    }
}

fn chapter_info() -> ChapterInfo {
    //
    let time = OffsetDateTime::now_utc();

    ChapterInfo {
        id: "chapter-1".to_string(),
        comic_id: "comic-1".to_string(),
        comic: None,
        is_pinned: true,
        index: 0,
        subtitle: "Chapter One".to_string(),
        page_count: 1,
        total_unit_count: 1,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        stages: StageMask::try_from(0).ok().unwrap(),
        creator_id: "creator-user".to_string(),
        creator: None,
        created_at: time,
        updated_at: time,
    }
}

fn assignment_info(id: &str, user_id: &str, roles: RoleMask) -> AssignmentInfo {
    //
    let time = OffsetDateTime::now_utc();

    AssignmentInfo {
        id: id.to_string(),
        chapter_id: "chapter-1".to_string(),
        user_id: user_id.to_string(),
        user: None,
        chapter: None,
        roles,
        created_at: time,
        updated_at: time,
    }
}

fn seed_chapter_scope(mock: &Mock) {
    //
    mock.seed_team(team_info());

    mock.seed_workset(workset_info());

    mock.seed_comic(comic_info());

    mock.seed_chapter(chapter_info());
}

#[tokio::test]
async fn develop_dispatches_user_signup() {
    //
    let mock = Arc::new(Mock::new());

    mock.seed_team(team_info());

    let develop = AsyncEffectDevelop::new(Arc::clone(&mock), 8);

    EffectDevelop::develop(
        &develop,
        Event::UserSignedUp(UserSignedUpPayload {
            team_id: "team-1".to_string(),
            invitor_id: "user-owner".to_string(),
            invitee_qid: "10001".to_string(),
        }),
    )
    .await;

    develop.close().await;

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.system_mails.len(), 1);

    assert_eq!(snapshot.system_mails[0].receiver_id, "user-owner");
}

#[tokio::test]
async fn develop_dispatches_chapter_workflow_completed() {
    //
    let mock = Arc::new(Mock::new());

    seed_chapter_scope(&mock);

    mock.seed_assignment(assignment_info(
        "assignment-proofreader",
        "proofreader-user",
        RoleMask::from(RoleField::PROOFREADER),
    ));

    mock.seed_assignment(assignment_info(
        "assignment-reviewer",
        "reviewer-user",
        RoleMask::from(RoleField::REVIEWER),
    ));

    let develop = AsyncEffectDevelop::new(Arc::clone(&mock), 8);

    EffectDevelop::develop(
        &develop,
        Event::ChapterWorkflowCompleted(ChapterWorkflowCompletedPayload {
            chapter_id: "chapter-1".to_string(),
            completed_stage: Stage::Translate,
        }),
    )
    .await;

    develop.close().await;

    let snapshot = mock.snapshot();

    let mut receiver_ids = snapshot
        .system_mails
        .iter()
        .map(|system_mail| system_mail.receiver_id.as_str())
        .collect::<Vec<_>>();

    receiver_ids.sort_unstable();

    assert_eq!(receiver_ids, vec!["proofreader-user", "reviewer-user"]);
}

#[tokio::test]
async fn develop_dispatches_chapter_published() {
    //
    let mock = Arc::new(Mock::new());

    seed_chapter_scope(&mock);

    mock.seed_assignment(assignment_info(
        "assignment-reviewer",
        "reviewer-user",
        RoleMask::from(RoleField::REVIEWER),
    ));

    let develop = AsyncEffectDevelop::new(Arc::clone(&mock), 8);

    EffectDevelop::develop(
        &develop,
        Event::ChapterPublished(ChapterPublishedPayload {
            chapter_id: "chapter-1".to_string(),
        }),
    )
    .await;

    develop.close().await;

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.system_mails.len(), 1);

    assert_eq!(snapshot.system_mails[0].receiver_id, "reviewer-user");
}

#[tokio::test]
async fn close_is_idempotent() {
    //
    let mock = Arc::new(Mock::new());

    let develop = AsyncEffectDevelop::new(mock, 8);

    develop.close().await;

    develop.close().await;
}
