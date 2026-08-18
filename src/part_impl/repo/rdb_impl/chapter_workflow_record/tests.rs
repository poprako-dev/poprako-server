//! RDB coverage for immutable chapter workflow record persistence.

use poprako_orchestra::{Nucl as _, Run as _, Step as _};
use time::{Duration, OffsetDateTime};

use crate::model::read::spec::chapter_workflow_record::ChapterWorkflowRecordListSpec;
use crate::model::write::chapter_workflow_record::ChapterWorkflowRecordEntry;
use crate::part::nucl::RepeatableRead;
use crate::part::repo::oper::chapter_workflow_record::{
    CreateChapterWorkflowRecords, DeleteChapterWorkflowRecords,
    ListChapterWorkflowRecordInfos,
};
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::test_shared;
use crate::result::BaseError;
use crate::shared::RdbCore;
use crate::value::chapter::{Stage, StagePhase};
use crate::value::chapter_workflow_record::{
    ChapterWorkflowRecordOrigin, ChapterWorkflowRecordPayload,
};

const PREFIX: &str = "rdb-test-chapter-workflow-record-domain-";

/// Verifies JSONB payloads, deterministic paging, and explicit deletion.
pub async fn chapter_workflow_record_roundtrip_uses_testcontainer(
    shared: RdbCore,
) {
    test_shared::reset(&shared, PREFIX).await;

    let chapter_fixture = test_shared::seed_chapter(&shared, PREFIX).await;

    let repo = HybRepo::new(shared.clone());

    let nucl = RdbNucl::<RepeatableRead>::new(shared.clone());

    let created_at = OffsetDateTime::UNIX_EPOCH + Duration::seconds(1);

    let entries = [
        ChapterWorkflowRecordEntry {
            id: format!("{}record-a", PREFIX),
            chapter_id: chapter_fixture.chapter_entry.id.clone(),
            actor_user_id: Some(chapter_fixture.creator_form.id.clone()),
            payload: ChapterWorkflowRecordPayload::ChapterCreated,
            created_at,
        },
        ChapterWorkflowRecordEntry {
            id: format!("{}record-b", PREFIX),
            chapter_id: chapter_fixture.chapter_entry.id.clone(),
            actor_user_id: Some(chapter_fixture.creator_form.id.clone()),
            payload: ChapterWorkflowRecordPayload::StageTransitioned {
                stage: Stage::Translate,
                previous_phase: StagePhase::Pending,
                next_phase: StagePhase::Active,
                origin: ChapterWorkflowRecordOrigin::UnitEdit,
            },
            created_at,
        },
        ChapterWorkflowRecordEntry {
            id: format!("{}record-c", PREFIX),
            chapter_id: chapter_fixture.chapter_entry.id.clone(),
            actor_user_id: None,
            payload: ChapterWorkflowRecordPayload::ChapterPinned,
            created_at: OffsetDateTime::UNIX_EPOCH,
        },
    ];

    nucl.coord(async |context| {
        repo.step(context, &CreateChapterWorkflowRecords { entries: &entries })
            .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .ok()
    .unwrap();

    let list_spec = ChapterWorkflowRecordListSpec {
        chapter_id: chapter_fixture.chapter_entry.id.clone(),
        offset: 0,
        limit: 2,
    };

    let latest_record_infos = repo
        .run(&ListChapterWorkflowRecordInfos { spec: &list_spec })
        .await
        .ok()
        .unwrap();

    assert_eq!(latest_record_infos.len(), 2);

    assert_eq!(latest_record_infos[0].id, entries[1].id);

    assert_eq!(latest_record_infos[1].id, entries[0].id);

    assert!(matches!(
        &latest_record_infos[0].payload,
        ChapterWorkflowRecordPayload::StageTransitioned {
            origin: ChapterWorkflowRecordOrigin::UnitEdit,
            ..
        }
    ));

    nucl.coord(async |context| {
        repo.step(
            context,
            &DeleteChapterWorkflowRecords {
                chapter_id: &chapter_fixture.chapter_entry.id,
            },
        )
        .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .ok()
    .unwrap();

    let remaining_record_infos = repo
        .run(&ListChapterWorkflowRecordInfos { spec: &list_spec })
        .await
        .ok()
        .unwrap();

    assert!(remaining_record_infos.is_empty());

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
