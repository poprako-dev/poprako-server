// comic_archive_roundtrip_reads_test_database_url(GetComicArchiveSnapshotExcluded, CommitComicArchive)(positive): archive rows persist as decodable bytes while active data is removed without changing workset counts.

use super::*;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use poprako_orchestra::Nucl as _;
use time::OffsetDateTime;

use crate::complex::comic_archive::ComicArchiveComplex;
use crate::model::comic_archive::{ArchivedComicPayload, ComicArchiveWrite};
use crate::part::repo::oper::comic_archive::{
    CommitComicArchive, GetComicArchiveSnapshotExcluded,
};
use crate::part_impl::drive::rdb_impl::RdbDrive;
use crate::part_impl::repo::rdb_impl::schema::{
    t_chapter, t_comic, t_comic_archive, t_page, t_workset,
};
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};
use crate::result::BaseError;
use crate::util::decompress_archive;

const PREFIX: &str = "rdb-test-comic-archive-domain-";

#[tokio::test]
async fn comic_archive_roundtrip_reads_test_database_url() {
    //
    let shared = test_shared::shared().await;

    test_shared::reset(&shared, PREFIX).await;

    let page_fixture = test_shared::seed_page(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let drive = RdbDrive::new(shared.clone());

    let source_comic_id = page_fixture.chapter_entry.comic_id.clone();

    let archiver_id = page_fixture.chapter_entry.creator_id.clone();

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

    let comic_archive_write = drive
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

            let (comic_archive_write, _) = ComicArchiveComplex::prepare_write(
                comic_archive_snapshot,
                archiver_id.clone(),
                OffsetDateTime::now_utc(),
            )
            .await?;

            repo.step(
                context,
                &CommitComicArchive {
                    write: &comic_archive_write,
                },
            )
            .await?;

            Ok::<ComicArchiveWrite, BaseError>(comic_archive_write)
        })
        .await
        .ok()
        .unwrap();

    let mut conn = shared.get().await.unwrap();

    let (
        archive_team_id,
        comic_archived_bytes,
        comic_archiver_id,
        comic_created_at,
    ) = t_comic_archive::table
        .filter(t_comic_archive::f_id.eq(&comic_archive_write.record.id))
        .select((
            t_comic_archive::f_team_id,
            t_comic_archive::f_archived_bytes,
            t_comic_archive::f_archiver_id,
            t_comic_archive::f_created_at,
        ))
        .first::<(String, Vec<u8>, String, OffsetDateTime)>(&mut conn)
        .await
        .unwrap();

    let workset_comic_count_after = t_workset::table
        .filter(t_workset::f_id.eq(&archive_workset_id))
        .select(t_workset::f_comic_count)
        .first::<i32>(&mut conn)
        .await
        .unwrap();

    let archived_comic_payload: ArchivedComicPayload =
        decompress_archive(&comic_archived_bytes).unwrap();

    assert_eq!(archive_team_id, page_fixture.team_entry.id);

    assert_eq!(comic_archiver_id, archiver_id);

    assert_eq!(comic_created_at, comic_archive_write.record.created_at);

    assert_eq!(archived_comic_payload.source_comic_id, source_comic_id);

    assert_eq!(
        archived_comic_payload.chapters[0].source_chapter_id,
        page_fixture.chapter_entry.id
    );

    assert_eq!(archived_comic_payload.chapters[0].pages.len(), 1);

    assert_eq!(
        archived_comic_payload.chapters[0].pages[0].source_page_id,
        page_fixture.page_entry.id
    );

    assert_eq!(
        t_comic::table
            .filter(t_comic::f_id.eq(&source_comic_id))
            .count()
            .get_result::<i64>(&mut conn)
            .await
            .unwrap(),
        0
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
            .filter(t_comic_archive::f_id.eq(&comic_archive_write.record.id)),
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
