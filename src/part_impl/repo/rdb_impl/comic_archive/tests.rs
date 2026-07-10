// comic_archive_roundtrip_reads_test_database_url(ComicArchiveStep)(positive): archive rows persist as decodable bytes while active data is removed without changing workset counts.

use super::*;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;

use crate::complex::comic_archive::ComicArchiveComplex;
use crate::model::comic_archive::{
    ArchivedChapterPayload, ArchivedComicPayload, ArchivedTranslationPayload,
    ComicArchiveWrite,
};
use crate::part::repo::step::comic_archive::ComicArchiveStep;
use crate::part_impl::drive::rdb_impl::RdbDrive;
use crate::part_impl::repo::rdb_impl::{
    RdbRepo, t_archived_chapter, t_archived_comic, t_archived_translation,
    t_chapter, t_comic, t_page, t_workset, test_shared,
};
use crate::result::RegularError;
use crate::util::{DeriveTransactional as _, decompress_archive};

const PREFIX: &str = "rdb-test-comic-archive-domain-";

#[tokio::test]
async fn comic_archive_roundtrip_reads_test_database_url() {
    let shared = test_shared::shared().await;

    test_shared::reset(&shared, PREFIX).await;

    let page_fixture = test_shared::seed_page(&shared, PREFIX).await;
    let repo = RdbRepo::new(shared.clone());
    let drive = RdbDrive::new(shared.clone());
    let transactional_repo = repo.derive_transactional().await;
    let source_comic_id = page_fixture.chapter_form.comic_id.clone();
    let archiver_id = page_fixture.chapter_form.creator_id.clone();

    let (archive_workset_id, workset_comic_count_before) = {
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
        .with_context(async |context| {
            let comic_archive_snapshot = Advance::advance(
                &transactional_repo,
                context,
                &ComicArchiveStep::lock_snapshot(&source_comic_id),
            )
            .await?;
            let comic_archive_write = ComicArchiveComplex::build_write(
                comic_archive_snapshot,
                archiver_id.clone(),
                OffsetDateTime::now_utc(),
            )?;

            Advance::advance(
                &transactional_repo,
                context,
                &ComicArchiveStep::commit(&comic_archive_write),
            )
            .await?;

            Ok::<ComicArchiveWrite, RegularError>(comic_archive_write)
        })
        .await
        .ok()
        .unwrap();

    let mut conn = shared.get().await.unwrap();
    let (comic_archived_bytes, comic_archiver_id, comic_created_at) =
        t_archived_comic::table
            .filter(
                t_archived_comic::f_id.eq(&comic_archive_write.comic_record.id),
            )
            .select((
                t_archived_comic::f_archived_bytes,
                t_archived_comic::f_archiver_id,
                t_archived_comic::f_created_at,
            ))
            .first::<(Vec<u8>, String, OffsetDateTime)>(&mut conn)
            .await
            .unwrap();
    let chapter_archived_bytes = t_archived_chapter::table
        .filter(
            t_archived_chapter::f_id
                .eq(&comic_archive_write.chapter_records[0].id),
        )
        .select(t_archived_chapter::f_archived_bytes)
        .first::<Vec<u8>>(&mut conn)
        .await
        .unwrap();
    let translation_archived_bytes = t_archived_translation::table
        .filter(
            t_archived_translation::f_id
                .eq(&comic_archive_write.translation_records[0].id),
        )
        .select(t_archived_translation::f_archived_bytes)
        .first::<Vec<u8>>(&mut conn)
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
    let archived_chapter_payload: ArchivedChapterPayload =
        decompress_archive(&chapter_archived_bytes).unwrap();
    let archived_translation_payload: ArchivedTranslationPayload =
        decompress_archive(&translation_archived_bytes).unwrap();

    assert_eq!(comic_archiver_id, archiver_id);
    assert_eq!(
        comic_created_at,
        comic_archive_write.comic_record.created_at
    );
    assert_eq!(archived_comic_payload.source_comic_id, source_comic_id);
    assert_eq!(
        archived_chapter_payload.source_chapter_id,
        page_fixture.chapter_form.id
    );
    assert_eq!(
        archived_translation_payload.source_chapter_id,
        page_fixture.chapter_form.id
    );
    assert_eq!(archived_translation_payload.pages.len(), 1);
    assert_eq!(
        archived_translation_payload.pages[0].source_page_id,
        page_fixture.page_form.id
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
            .filter(t_chapter::f_id.eq(&page_fixture.chapter_form.id))
            .count()
            .get_result::<i64>(&mut conn)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        t_page::table
            .filter(t_page::f_id.eq(&page_fixture.page_form.id))
            .count()
            .get_result::<i64>(&mut conn)
            .await
            .unwrap(),
        0
    );
    assert_eq!(workset_comic_count_after, workset_comic_count_before);

    diesel::delete(
        t_archived_translation::table.filter(
            t_archived_translation::f_id
                .eq(&comic_archive_write.translation_records[0].id),
        ),
    )
    .execute(&mut conn)
    .await
    .unwrap();
    diesel::delete(t_archived_chapter::table.filter(
        t_archived_chapter::f_id.eq(&comic_archive_write.chapter_records[0].id),
    ))
    .execute(&mut conn)
    .await
    .unwrap();
    diesel::delete(t_archived_comic::table.filter(
        t_archived_comic::f_id.eq(&comic_archive_write.comic_record.id),
    ))
    .execute(&mut conn)
    .await
    .unwrap();

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
