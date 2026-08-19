//! Permanent comic archive commit operations.

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;
use tracing::instrument;

use crate::model::write::comic_archive::ComicArchiveEntry;
use crate::part_impl::repo::rdb_impl::entity::comic_archive::ComicArchiveEntryRow;
use crate::part_impl::repo::rdb_impl::schema::t_assignment::dsl::{f_chapter_id as assignment_chapter_id, t_assignment};
use crate::part_impl::repo::rdb_impl::schema::t_assignment_invitation::dsl::{f_chapter_id as invitation_chapter_id, t_assignment_invitation};
use crate::part_impl::repo::rdb_impl::schema::t_chapter::dsl::{f_id as chapter_id, t_chapter};
use crate::part_impl::repo::rdb_impl::schema::t_chapter_workflow_record::dsl::{f_chapter_id as workflow_record_chapter_id, t_chapter_workflow_record};
use crate::part_impl::repo::rdb_impl::schema::t_comic::dsl::{f_archived_at as comic_archived_at, f_cover_extension as comic_cover_extension, f_cover_hash as comic_cover_hash, f_cover_key as comic_cover_key, f_cover_uploaded as comic_cover_uploaded, f_cover_version as comic_cover_version, f_id as comic_id, f_updated_at as comic_updated_at, t_comic};
use crate::part_impl::repo::rdb_impl::schema::t_comic_archive;
use crate::part_impl::repo::rdb_impl::schema::t_page::dsl::{f_chapter_id as page_chapter_id, t_page};
use crate::part_impl::repo::rdb_impl::schema::t_term::dsl::{f_termbase_id as term_termbase_id, t_term};
use crate::part_impl::repo::rdb_impl::schema::t_termbase::dsl::{f_comic_id as termbase_comic_id, f_id as termbase_id, t_termbase};
use crate::part_impl::repo::rdb_impl::schema::t_unit::dsl::{f_page_id as unit_page_id, t_unit};
use crate::result::{BaseRest, accept};
use crate::shared::RdbConn;
use crate::shared::result::diesel;

// Store archive payload, clear sources, and retain the comic management row.
#[instrument(level = "info", skip_all)]
/// Commits one permanent archive and clears its active source resources.
pub async fn commit(
    conn: &mut RdbConn,
    comic_archive_entry: &ComicArchiveEntry,
) -> BaseRest<()> {
    //
    let comic_archive_row =
        ComicArchiveEntryRow::from(&comic_archive_entry.record);

    diesel::insert_into(t_comic_archive::table)
        .values(&comic_archive_row)
        .execute(conn)
        .await
        .map_err(diesel)?;

    let now = OffsetDateTime::now_utc();

    diesel::update(
        t_comic.filter(comic_id.eq(&comic_archive_entry.source_comic_id)),
    )
    .set((
        comic_archived_at.eq(Some(now)),
        comic_cover_key.eq(None::<String>),
        comic_cover_uploaded.eq(None::<bool>),
        comic_cover_version.eq(None::<i64>),
        comic_cover_hash.eq(None::<Vec<u8>>),
        comic_cover_extension.eq(None::<String>),
        comic_updated_at.eq(now),
    ))
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::delete(t_assignment_invitation.filter(
        invitation_chapter_id.eq_any(&comic_archive_entry.source_chapter_ids),
    ))
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::delete(t_assignment.filter(
        assignment_chapter_id.eq_any(&comic_archive_entry.source_chapter_ids),
    ))
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::delete(
        t_chapter_workflow_record.filter(
            workflow_record_chapter_id
                .eq_any(&comic_archive_entry.source_chapter_ids),
        ),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    let termbase_ids = t_termbase
        .filter(termbase_comic_id.eq(&comic_archive_entry.source_comic_id))
        .select(termbase_id)
        .load::<String>(conn)
        .await
        .map_err(diesel)?;

    diesel::delete(t_term.filter(term_termbase_id.eq_any(&termbase_ids)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    diesel::delete(
        t_termbase
            .filter(termbase_comic_id.eq(&comic_archive_entry.source_comic_id)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::delete(
        t_unit
            .filter(unit_page_id.eq_any(&comic_archive_entry.source_page_ids)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::delete(t_page.filter(
        page_chapter_id.eq_any(&comic_archive_entry.source_chapter_ids),
    ))
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::delete(
        t_chapter
            .filter(chapter_id.eq_any(&comic_archive_entry.source_chapter_ids)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    accept(())
}
