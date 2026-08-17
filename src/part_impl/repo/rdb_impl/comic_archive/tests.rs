// comic_archive_roundtrip_uses_testcontainer(GetComicArchiveSnapshotExcluded, CommitComicArchive)(positive): archive rows persist as decodable bytes while the archived comic marker remains and active descendants are removed without changing workset counts.

use super::*;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use poprako_orchestra::Nucl as _;
use time::OffsetDateTime;

use crate::complex::comic_archive::ComicArchiveComplex;
use crate::model::write::chapter_workflow_record::ChapterWorkflowRecordEntry;
use crate::model::write::comic_archive::ComicArchiveEntry;
use crate::part::repo::oper::chapter_workflow_record::CreateChapterWorkflowRecords;
use crate::part::repo::oper::comic_archive::{
    CommitComicArchive, GetComicArchiveSnapshotExcluded,
};
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::schema::{
    t_chapter, t_comic, t_comic_archive, t_page, t_workset,
};
use crate::part_impl::repo::rdb_impl::test_shared;
use crate::result::BaseError;
use crate::shared::RdbCore;
use crate::value::chapter_workflow_record::ChapterWorkflowRecordPayload;

const PREFIX: &str = "rdb-test-comic-archive-domain-";

/// Verifies comic archive roundtrip via testcontainers.
/// Verifies comic archive roundtrip via testcontainers.
pub async fn comic_archive_roundtrip_uses_testcontainer(shared: RdbCore) {
    //
    test_shared::reset(&shared, PREFIX).await;

    let page_fixture = test_shared::seed_page(&shared, PREFIX).await;

    let repo = HybRepo::new(shared.clone());

    let nucl =
        RdbNucl::<crate::part::nucl::RepeatableRead>::new(shared.clone());

    let source_comic_id = page_fixture.chapter_entry.comic_id.clone();

    let archiver_id = page_fixture.chapter_entry.creator_id.clone();

    let workflow_record_entry = ChapterWorkflowRecordEntry {
        id: format!("{}workflow-record", PREFIX),
        chapter_id: page_fixture.chapter_entry.id.clone(),
        actor_user_id: Some(archiver_id.clone()),
        payload: ChapterWorkflowRecordPayload::ChapterSubtitleUpdated {
            previous_subtitle: "before archive".into(),
            next_subtitle: "after archive".into(),
        },
        created_at: OffsetDateTime::UNIX_EPOCH,
    };

    nucl.coord(async |context| {
        repo.step(
            context,
            &CreateChapterWorkflowRecords {
                entries: std::slice::from_ref(&workflow_record_entry),
            },
        )
        .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .ok()
    .unwrap();

    let (archive_workset_id, workset_comic_count_before) = {
        //
        let mut conn = shared.get().await.unwrap();

        t_workset::table
            .inner_join(
                t_comic::table.on(t_comic::f_workset_id.eq(t_workset::f_id)),
            )
            .filter(t_comic::f_id.eq(&source_comic_id))
            .select((t_workset::f_id, t_workset::f_comic_count))
            .first::<(String, i32)>(&mut conn)
            .await
            .unwrap()
    };

    let comic_archive_entry = nucl
        .coord(async |context| {
            //
            let comic_archive_snapshot = repo
                .step(
                    context,
                    &GetComicArchiveSnapshotExcluded {
                        comic_id: &source_comic_id,
                    },
                )
                .await?;

            let (comic_archive_entry, _) = ComicArchiveComplex::prepare_entry(
                comic_archive_snapshot,
                archiver_id.clone(),
                OffsetDateTime::now_utc(),
            )
            .await?;

            repo.step(
                context,
                &CommitComicArchive {
                    entry: &comic_archive_entry,
                },
            )
            .await?;

            Ok::<ComicArchiveEntry, BaseError>(comic_archive_entry)
        })
        .await
        .ok()
        .unwrap();

    let mut conn = shared.get().await.unwrap();

    let (
        archive_team_id,
        archive_source_comic_id,
        comic_archived_payload,
        comic_archiver_id,
        comic_created_at,
    ) = t_comic_archive::table
        .filter(t_comic_archive::f_id.eq(&comic_archive_entry.record.id))
        .select((
            t_comic_archive::f_team_id,
            t_comic_archive::f_source_comic_id,
            t_comic_archive::f_archived_payload,
            t_comic_archive::f_archiver_id,
            t_comic_archive::f_created_at,
        ))
        .first::<(String, String, String, String, OffsetDateTime)>(&mut conn)
        .await
        .unwrap();

    let workset_comic_count_after = t_workset::table
        .filter(t_workset::f_id.eq(&archive_workset_id))
        .select(t_workset::f_comic_count)
        .first::<i32>(&mut conn)
        .await
        .unwrap();

    let archived_comic_payload: serde_json::Value =
        serde_json::from_str(&comic_archived_payload).unwrap();

    assert_eq!(archive_team_id, page_fixture.team_entry.id);

    assert_eq!(archive_source_comic_id, source_comic_id.as_str());

    assert_eq!(comic_archiver_id, archiver_id);

    assert_eq!(comic_created_at, comic_archive_entry.record.created_at);

    assert_eq!(archived_comic_payload["source_comic_id"], source_comic_id);

    assert_eq!(
        archived_comic_payload["chapters"][0]["source_chapter_id"],
        page_fixture.chapter_entry.id
    );

    assert_eq!(
        archived_comic_payload["chapters"][0]["workflow_records"][0]["id"],
        workflow_record_entry.id
    );

    assert_eq!(
        archived_comic_payload["chapters"][0]["workflow_records"][0]["kind"],
        "chapter-subtitle-updated"
    );

    assert_eq!(
        archived_comic_payload["chapters"][0]["workflow_records"][0]["payload"]
            ["previous_subtitle"],
        "before archive"
    );

    assert_eq!(
        archived_comic_payload["chapters"][0]["pages"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    assert_eq!(
        archived_comic_payload["chapters"][0]["pages"][0]["source_page_id"],
        page_fixture.page_entry.id
    );

    assert_eq!(
        t_comic::table
            .filter(t_comic::f_id.eq(&source_comic_id))
            .count()
            .get_result::<i64>(&mut conn)
            .await
            .unwrap(),
        1
    );

    assert_eq!(
        t_chapter::table
            .filter(t_chapter::f_id.eq(&page_fixture.chapter_entry.id))
            .count()
            .get_result::<i64>(&mut conn)
            .await
            .unwrap(),
        0
    );

    assert_eq!(
        t_page::table
            .filter(t_page::f_id.eq(&page_fixture.page_entry.id))
            .count()
            .get_result::<i64>(&mut conn)
            .await
            .unwrap(),
        0
    );

    assert_eq!(workset_comic_count_after, workset_comic_count_before);

    diesel::delete(
        t_comic_archive::table
            .filter(t_comic_archive::f_id.eq(&comic_archive_entry.record.id)),
    )
    .execute(&mut conn)
    .await
    .unwrap();

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
