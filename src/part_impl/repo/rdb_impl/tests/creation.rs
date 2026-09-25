//! Chapter creation contracts exercised through use cases and real PostgreSQL.

mod failure;
mod snapshot;

// comic_creation_history_failure_rolls_back_workset(create)(negative): a late PostgreSQL history failure rolls back Comic, Chapter, assignments and Workset allocation.
// chapter_creation_history_failure_restores_siblings(create)(negative): a late PostgreSQL history failure restores existing pins, counts, cursors, activity and history.

use diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{Nucl as _, Run as _, Step as _};
use time::OffsetDateTime;

use poprako_rdb_core::RdbCore;

use crate::data::instr::chapter::CreateChapterInstr;
use crate::data::instr::comic::CreateComicInstr;
use crate::model::read::spec::chapter_workflow_record::ChapterWorkflowRecordListSpec;
use crate::model::shared::user::UserToken;
use crate::model::write::member::MemberEntry;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::assignment::ListAssignmentInfos;
use crate::part::repo::oper::chapter::GetChapterInfo;
use crate::part::repo::oper::chapter_workflow_record::ListChapterWorkflowRecordInfos;
use crate::part::repo::oper::comic::GetComicInfo;
use crate::part::repo::oper::member::CreateMember;
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::schema::t_comic;
use crate::part_impl::repo::rdb_impl::test_shared;
use crate::part_impl::repo::rdb_impl::tests::creation::failure::DuplicateHistoryRepo;
use crate::part_impl::repo::rdb_impl::tests::creation::snapshot::CreationSnapshot;
use crate::result::{BaseError, ExpectedVariant, accept};
use crate::shared::test_rdb::start;
use crate::usecase::{chapter as chapter_usecase, comic as comic_usecase};
use crate::value::chapter_workflow_record::ChapterWorkflowRecordPayload;
use crate::value::pagination::PubListLimit;
use crate::value::role::{RoleField, RoleMask};

// Seeds a real Workset with an administrator who can explicitly translate.
async fn seed_admin_workset(shared: &RdbCore) -> (String, UserToken) {
    //
    let prefix = "rdb-creation-";

    let workset_fixture = test_shared::seed_workset(shared, prefix).await;

    let user_entry = test_shared::form::user_entry(prefix, "owner");

    let member_entry = MemberEntry {
        id: format!("{prefix}member"),
        user_id: user_entry.id.clone(),
        user_nickname: user_entry.nickname,
        team_id: workset_fixture.team_entry.id,
        roles: RoleMask::from(RoleField::ADMIN)
            .union(RoleMask::from(RoleField::TRANSLATOR)),
    };

    let repo = HybRepo::new(shared.clone());

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    nucl.coord(async |context| {
        //
        repo.step(
            context,
            &CreateMember {
                entry: &member_entry,
            },
        )
        .await?;

        accept(())
    })
    .await
    .unwrap();

    let token = UserToken {
        user_id: user_entry.id,
    };

    (workset_fixture.workset_entry.id, token)
}

// Builds an explicit-subtitle Comic request without implicitly assigning work.
fn comic_instr(workset_id: &str) -> CreateComicInstr {
    //
    CreateComicInstr {
        workset_id: workset_id.to_owned(),
        title: "Creation transaction".into(),
        author: "Author".into(),
        description: None,
        first_chapter_subtitle: Some("Opening".into()),
        preset_assignment_roles: None,
    }
}

// Verifies exact history payloads and the actor through the existing read operation.
async fn assert_history(
    repo: &HybRepo,
    chapter_id: &str,
    actor_id: &str,
    expected: &[ChapterWorkflowRecordPayload],
) {
    //
    let spec = ChapterWorkflowRecordListSpec {
        chapter_id: chapter_id.to_owned(),
        offset: 0,
        limit: PubListLimit::new(10).unwrap(),
    };

    let records = repo
        .run(&ListChapterWorkflowRecordInfos { spec: &spec })
        .await
        .unwrap();

    assert_eq!(records.len(), expected.len());

    for (record, payload) in records.iter().zip(expected) {
        //
        assert_eq!(&record.payload, payload);

        assert_eq!(record.actor_user_id.as_deref(), Some(actor_id));
    }
}

#[tokio::test]
#[serial_test::serial(repo_rdb)]
async fn comic_creation_history_failure_rolls_back_workset() {
    //
    let test_rdb = start().await;

    let shared = test_rdb.core();

    let (workset_id, token) = seed_admin_workset(&shared).await;

    let repo = HybRepo::new(shared.clone());

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let before = CreationSnapshot::load(&shared, &workset_id).await;

    let failure_repo = DuplicateHistoryRepo::new(repo.clone());

    let mut instr = comic_instr(&workset_id);

    instr.preset_assignment_roles = Some(RoleField::TRANSLATOR.into());

    let result =
        comic_usecase::create((&nucl, &failure_repo), token.clone(), instr)
            .await;

    assert!(matches!(
        result,
        Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            ..
        })
    ));

    assert!(failure_repo.history_written());

    assert_eq!(CreationSnapshot::load(&shared, &workset_id).await, before);

    let instr = comic_instr(&workset_id);

    let created = comic_usecase::create((&nucl, &repo), token.clone(), instr)
        .await
        .unwrap();

    let comic_info = repo
        .run(&GetComicInfo {
            id: &created.id,
            incls: &[],
        })
        .await
        .unwrap();

    assert_eq!(comic_info.index, 0);

    assert_eq!(comic_info.chapter_count, 1);

    let workset_info =
        repo.run(&GetWorksetInfo { id: &workset_id }).await.unwrap();

    assert_eq!(workset_info.comic_count, 1);

    let chapter_info = repo
        .run(&GetChapterInfo {
            id: &created.chapter_id,
            incls: &[],
        })
        .await
        .unwrap();

    assert_eq!(chapter_info.index, 0);

    assert_eq!(chapter_info.comic_id, created.id);

    assert_eq!(chapter_info.subtitle, "Opening");

    assert!(chapter_info.is_pinned);

    let assignments = repo
        .run(&ListAssignmentInfos::Chapter {
            chapter_id: &created.chapter_id,
            role: None,
            incls: &[],
        })
        .await
        .unwrap();

    assert!(assignments.is_empty());

    assert_history(
        &repo,
        &created.chapter_id,
        &token.user_id,
        &[ChapterWorkflowRecordPayload::ChapterCreated],
    )
    .await;
}

#[tokio::test]
#[serial_test::serial(repo_rdb)]
async fn chapter_creation_history_failure_restores_siblings() {
    //
    let test_rdb = start().await;

    let shared = test_rdb.core();

    let (workset_id, token) = seed_admin_workset(&shared).await;

    let repo = HybRepo::new(shared.clone());

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let instr = comic_instr(&workset_id);

    let created = comic_usecase::create((&nucl, &repo), token.clone(), instr)
        .await
        .unwrap();

    let instr = CreateChapterInstr {
        comic_id: created.id.clone(),
        subtitle: Some("Second".into()),
        preset_assignment_roles: Some(RoleField::TRANSLATOR.into()),
    };

    let second = chapter_usecase::create((&nucl, &repo), token.clone(), instr)
        .await
        .unwrap();

    // A fixed old activity time makes the later activity assertion deterministic.
    {
        let mut conn = shared.get().await.unwrap();

        diesel::update(t_comic::table.find(&created.id))
            .set(t_comic::f_last_active_at.eq(OffsetDateTime::UNIX_EPOCH))
            .execute(&mut conn)
            .await
            .unwrap();
    }

    let before = CreationSnapshot::load(&shared, &workset_id).await;

    let failure_repo = DuplicateHistoryRepo::new(repo.clone());

    let instr = CreateChapterInstr {
        comic_id: created.id.clone(),
        subtitle: None,
        preset_assignment_roles: Some(RoleField::TRANSLATOR.into()),
    };

    let result =
        chapter_usecase::create((&nucl, &failure_repo), token.clone(), instr)
            .await;

    assert!(matches!(
        result,
        Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            ..
        })
    ));

    assert!(failure_repo.history_written());

    assert_eq!(CreationSnapshot::load(&shared, &workset_id).await, before);

    let instr = CreateChapterInstr {
        comic_id: created.id.clone(),
        subtitle: None,
        preset_assignment_roles: Some(RoleField::TRANSLATOR.into()),
    };

    let third = chapter_usecase::create((&nucl, &repo), token.clone(), instr)
        .await
        .unwrap();

    let chapter_info = repo
        .run(&GetChapterInfo {
            id: &third.id,
            incls: &[],
        })
        .await
        .unwrap();

    assert_eq!(chapter_info.index, 2);

    assert_eq!(chapter_info.comic_id, created.id);

    assert!(matches!(chapter_info.subtitle.as_str(), "第3话" | "Ch. 3"));

    assert!(chapter_info.is_pinned);

    let previous = repo
        .run(&GetChapterInfo {
            id: &second.id,
            incls: &[],
        })
        .await
        .unwrap();

    assert!(!previous.is_pinned);

    let comic_info = repo
        .run(&GetComicInfo {
            id: &created.id,
            incls: &[],
        })
        .await
        .unwrap();

    assert_eq!(comic_info.chapter_count, 3);

    assert!(comic_info.last_active_at > OffsetDateTime::UNIX_EPOCH);

    let assignments = repo
        .run(&ListAssignmentInfos::Chapter {
            chapter_id: &third.id,
            role: None,
            incls: &[],
        })
        .await
        .unwrap();

    assert_eq!(assignments.len(), 1);

    assert_eq!(assignments[0].user_id, token.user_id);

    assert_eq!(assignments[0].roles, RoleMask::from(RoleField::TRANSLATOR));

    assert_history(
        &repo,
        &third.id,
        &token.user_id,
        &[ChapterWorkflowRecordPayload::ChapterCreated],
    )
    .await;

    assert_history(
        &repo,
        &second.id,
        &token.user_id,
        &[
            ChapterWorkflowRecordPayload::ChapterUnpinned,
            ChapterWorkflowRecordPayload::ChapterCreated,
        ],
    )
    .await;
}
