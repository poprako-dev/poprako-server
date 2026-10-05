#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

// comic_archive_roundtrip_uses_testcontainer(GetComicArchiveSnapshotExcluded, CommitComicArchive)(positive): archive rows persist as decodable bytes while the archived comic marker remains and active descendants are removed without changing workset counts.

use super::*;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use poprako_orchestra::Nucl as _;
use time::OffsetDateTime;

use poprako_rdb_core::RdbCore;

use crate::complex::comic_archive as comic_archive_complex;
use crate::model::write::chapter_workflow_record::ChapterWorkflowRecordEntry;
use crate::model::write::comic_archive::ComicArchiveEntry;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::chapter_workflow_record::CreateChapterWorkflowRecords;
use crate::part::repo::oper::comic_archive::{
    CommitComicArchive, GetComicArchiveSnapshotExcluded,
};
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::schema::{
    t_chapter, t_comic, t_comic_archive, t_page, t_page_raw_ident, t_workset,
};
use crate::part_impl::repo::rdb_impl::test_shared;
use crate::part_impl::repo::rdb_impl::test_shared::PageFixture;
use crate::result::{BaseError, ExpectedVariant};
use crate::value::chapter_workflow_record::ChapterWorkflowRecordPayload;

const PREFIX: &str = "rdb-test-comic-archive-domain-";

// Verify archive metadata, retained comic and counters, removed descendants, and cleanup.
async fn verify_archive_storage(
    shared: &RdbCore,
    page_fixture: &PageFixture,
    comic_archive_entry: &ComicArchiveEntry,
    workflow_record_entry: &ChapterWorkflowRecordEntry<'_>,
    (archive_workset_id, workset_comic_count_before): (&str, i32),
) {
    let source_comic_id =
        page_fixture.chapter_entry.comic_id.clone().into_owned();

    let archiver_id =
        page_fixture.chapter_entry.creator_id.clone().into_owned();

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

    assert_eq!(archive_team_id, page_fixture.team_entry.id);

    assert_eq!(archive_source_comic_id, source_comic_id.as_str());

    assert_eq!(comic_archiver_id, archiver_id);

    assert_eq!(comic_created_at, comic_archive_entry.record.created_at);

    verify_archived_payload(
        &comic_archived_payload,
        page_fixture,
        workflow_record_entry,
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

    assert_eq!(
        t_page_raw_ident::table
            .filter(t_page_raw_ident::f_page_id.eq(&page_fixture.page_entry.id))
            .count()
            .get_result::<i64>(&mut conn)
            .await
            .unwrap(),
        0
    );

    diesel::delete(
        t_comic_archive::table
            .filter(t_comic_archive::f_id.eq(&comic_archive_entry.record.id)),
    )
    .execute(&mut conn)
    .await
    .unwrap();
}

// Verify the stored archive JSON contains chapter history and original page identity.
fn verify_archived_payload(
    comic_archived_payload: &str,
    page_fixture: &PageFixture,
    workflow_record_entry: &ChapterWorkflowRecordEntry<'_>,
) {
    let source_comic_id = page_fixture.chapter_entry.comic_id.as_ref();

    let archived_comic_payload: serde_json::Value =
        serde_json::from_str(comic_archived_payload).unwrap();

    let archived_chapter = archived_comic_payload
        .get("chapters")
        .unwrap()
        .get(0)
        .unwrap();

    let archived_page = archived_chapter.get("pages").unwrap().get(0).unwrap();

    let archived_workflow_record = archived_chapter
        .get("workflow_records")
        .unwrap()
        .get(0)
        .unwrap();

    assert_eq!(
        (*archived_comic_payload.get("source_comic_id").unwrap()),
        source_comic_id
    );

    assert_eq!(
        (*archived_chapter.get("source_chapter_id").unwrap()),
        page_fixture.chapter_entry.id
    );

    assert_eq!(
        (*archived_workflow_record.get("id").unwrap()),
        workflow_record_entry.id
    );

    assert_eq!(
        (*archived_workflow_record.get("kind").unwrap()),
        "chapter-subtitle-updated"
    );

    assert_eq!(
        (*archived_workflow_record
            .get("payload")
            .unwrap()
            .get("previous_subtitle")
            .unwrap()),
        "before archive"
    );

    assert_eq!(
        archived_chapter
            .get("pages")
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );

    assert_eq!(
        (*archived_page.get("source_page_id").unwrap()),
        page_fixture.page_entry.id
    );
}

/// Verifies comic archive roundtrip via testcontainers.
/// # Panics
/// Panics if fixture setup fails or a scenario assertion is violated.
#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
pub async fn comic_archive_roundtrip_uses_testcontainer(shared: RdbCore) {
    //
    test_shared::reset(&shared, PREFIX).await;

    let page_fixture = test_shared::seed_page(&shared, PREFIX).await;

    {
        let mut conn = shared.get().await.unwrap();

        diesel::insert_into(t_page_raw_ident::table)
            .values((
                t_page_raw_ident::f_page_id.eq(&page_fixture.page_entry.id),
                t_page_raw_ident::f_raw_ident.eq("source.png"),
            ))
            .execute(&mut conn)
            .await
            .unwrap();
    }

    let repo = HybRepo::new(shared.clone());

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let source_comic_id =
        page_fixture.chapter_entry.comic_id.clone().into_owned();

    let archiver_id =
        page_fixture.chapter_entry.creator_id.clone().into_owned();

    let workflow_record_entry = ChapterWorkflowRecordEntry {
        id: format!("{}workflow-record", PREFIX),
        chapter_id: page_fixture.chapter_entry.id.clone().into(),
        actor_user_id: Some(archiver_id.clone().into()),
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

            let comic_archive_entry = comic_archive_complex::prepare_entry(
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
        .unwrap();

    verify_archive_storage(
        &shared,
        &page_fixture,
        &comic_archive_entry,
        &workflow_record_entry,
        (&archive_workset_id, workset_comic_count_before),
    )
    .await;

    test_shared::cleanup(&shared, PREFIX).await.unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .unwrap();
}

/// Verifies tombstoned comics reject both snapshot loading and archive commit.
/// # Panics
/// Panics if fixture setup fails or a scenario assertion is violated.
pub async fn tombstoned_comic_rejects_archive(shared: RdbCore) {
    test_shared::reset(&shared, PREFIX).await;

    let page_fixture = test_shared::seed_page(&shared, PREFIX).await;

    let repo = HybRepo::new(shared.clone());

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let source_comic_id =
        page_fixture.chapter_entry.comic_id.clone().into_owned();

    let archiver_id =
        page_fixture.chapter_entry.creator_id.clone().into_owned();

    let comic_archive_snapshot = nucl
        .coord(async |context| {
            repo.step(
                context,
                &GetComicArchiveSnapshotExcluded {
                    comic_id: &source_comic_id,
                },
            )
            .await
        })
        .await
        .unwrap();

    let comic_archive_entry = comic_archive_complex::prepare_entry(
        comic_archive_snapshot,
        archiver_id,
        OffsetDateTime::now_utc(),
    )
    .await
    .unwrap();

    let mut conn = shared.get().await.unwrap();

    diesel::update(t_comic::table.filter(t_comic::f_id.eq(&source_comic_id)))
        .set(t_comic::f_deleted_at.eq(Some(OffsetDateTime::now_utc())))
        .execute(&mut conn)
        .await
        .unwrap();

    drop(conn);

    let snapshot_error = nucl
        .coord(async |context| {
            repo.step(
                context,
                &GetComicArchiveSnapshotExcluded {
                    comic_id: &source_comic_id,
                },
            )
            .await
        })
        .await
        .map_err(BaseError::from)
        .err()
        .unwrap();

    assert!(matches!(
        snapshot_error,
        BaseError::Expected {
            variant: ExpectedVariant::Args,
            ..
        }
    ));

    let commit_error = nucl
        .coord(async |context| {
            repo.step(
                context,
                &CommitComicArchive {
                    entry: &comic_archive_entry,
                },
            )
            .await
        })
        .await
        .map_err(BaseError::from)
        .err()
        .unwrap();

    assert!(matches!(
        commit_error,
        BaseError::Expected {
            variant: ExpectedVariant::Args,
            ..
        }
    ));

    let mut conn = shared.get().await.unwrap();

    let archive_count = t_comic_archive::table
        .filter(t_comic_archive::f_id.eq(&comic_archive_entry.record.id))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .unwrap();

    let chapter_count = t_chapter::table
        .filter(t_chapter::f_id.eq(&page_fixture.chapter_entry.id))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .unwrap();

    assert_eq!(archive_count, 0);
    assert_eq!(chapter_count, 1);

    drop(conn);

    test_shared::cleanup(&shared, PREFIX).await.unwrap();
}
