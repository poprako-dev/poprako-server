//! Permanent comic archive commit operations.

use diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
use diesel_async::RunQueryDsl as _;
use time::OffsetDateTime;
use tracing::instrument;

use poprako_rdb_core::RdbConn;
use poprako_util::i18n::trl;

use crate::model::write::comic_archive::ComicArchiveEntry;
use crate::part_impl::repo::rdb_impl::entity::comic_archive::ComicArchiveItem;
use crate::part_impl::repo::rdb_impl::schema::t_assignment::dsl::{f_chapter_id as assignment_chapter_id, t_assignment};
use crate::part_impl::repo::rdb_impl::schema::t_assignment_invitation::dsl::{f_chapter_id as invitation_chapter_id, t_assignment_invitation};
use crate::part_impl::repo::rdb_impl::schema::t_chapter::dsl::{f_id as chapter_id, t_chapter};
use crate::part_impl::repo::rdb_impl::schema::t_chapter_workflow_record::dsl::{f_chapter_id as workflow_record_chapter_id, t_chapter_workflow_record};
use crate::part_impl::repo::rdb_impl::schema::t_comic::dsl::{f_archived_at as comic_archived_at, f_deleted_at as comic_deleted_at, f_id as comic_id, f_updated_at as comic_updated_at, t_comic};
use crate::part_impl::repo::rdb_impl::schema::t_comic_archive;
use crate::part_impl::repo::rdb_impl::schema::t_page_source::dsl::{f_chapter_id as page_source_chapter_id, t_page_source};
use crate::part_impl::repo::rdb_impl::schema::t_term::dsl::{f_termbase_id as term_termbase_id, t_term};
use crate::part_impl::repo::rdb_impl::schema::t_termbase::dsl::{f_comic_id as termbase_comic_id, f_id as termbase_id, t_termbase};
use crate::part_impl::repo::rdb_impl::schema::t_unit::dsl::{f_page_source_id as unit_page_source_id, t_unit};
use crate::result::{BaseRest, ExpectedVariant};
use crate::shared::result::diesel;

// Store archive payload, clear sources, and retain the comic management record.
/// Commits one permanent archive and clears its active source resources.
#[instrument(level = "info", skip_all)]
pub async fn commit(
    conn: &mut RdbConn,
    entry: &ComicArchiveEntry,
) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    let updated_comic_count = diesel::update(
        t_comic
            .filter(comic_id.eq(&entry.record.source_comic_id))
            .filter(comic_deleted_at.is_null()),
    )
    .set((comic_archived_at.eq(Some(now)), comic_updated_at.eq(now)))
    .execute(conn)
    .await
    .map_err(diesel)?;

    if updated_comic_count != 1 {
        //
        return Err(expected_error! {
            ExpectedVariant::Args,
            message: trl("error-comic-not-found");
            error_variant = ?ExpectedVariant::Args,
            comic_id = %entry.record.source_comic_id,
            operation = "commit comic archive",
            "comic archive operation rejected",
        });
    }

    let item = ComicArchiveItem::from(&entry.record);

    diesel::insert_into(t_comic_archive::table)
        .values(&item)
        .execute(conn)
        .await
        .map_err(diesel)?;

    diesel::delete(
        t_assignment_invitation
            .filter(invitation_chapter_id.eq_any(&entry.source_chapter_ids)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::delete(
        t_assignment
            .filter(assignment_chapter_id.eq_any(&entry.source_chapter_ids)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::delete(
        t_chapter_workflow_record.filter(
            workflow_record_chapter_id.eq_any(&entry.source_chapter_ids),
        ),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    let termbase_ids = t_termbase
        .filter(termbase_comic_id.eq(&entry.record.source_comic_id))
        .select(termbase_id)
        .load::<String>(conn)
        .await
        .map_err(diesel)?;

    diesel::delete(t_term.filter(term_termbase_id.eq_any(&termbase_ids)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    diesel::delete(
        t_termbase.filter(termbase_comic_id.eq(&entry.record.source_comic_id)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::delete(
        t_unit
            .filter(unit_page_source_id.eq_any(&entry.source_page_source_ids)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::delete(
        t_page_source
            .filter(page_source_chapter_id.eq_any(&entry.source_chapter_ids)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::delete(
        t_chapter.filter(chapter_id.eq_any(&entry.source_chapter_ids)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    accept!(())
}
