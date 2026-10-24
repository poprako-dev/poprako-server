//! Transactional chapter revision updates.

use diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
use diesel_async::RunQueryDsl as _;

use poprako_rdb_core::RdbConn;

use crate::part_impl::repo::rdb_impl::schema::{t_chapter, t_page_artwork};
use crate::result::BaseRest;
use crate::shared::result::diesel as map_diesel;

/// Advances the chapter context under its existing write lock.
///
/// # Errors
/// Returns the mapped database error when revision persistence fails.
pub async fn bump_chapter(
    conn: &mut RdbConn,
    chapter_id: &str,
) -> BaseRest<()> {
    //
    diesel::update(t_chapter::table.find(chapter_id))
        .set(t_chapter::f_review_revision.eq(t_chapter::f_review_revision + 1))
        .execute(conn)
        .await
        .map_err(map_diesel)?;

    accept!(())
}

/// Invalidates every old review document after a complete import.
///
/// # Errors
/// Returns the mapped database error when revision persistence fails.
pub async fn renew_review(
    conn: &mut RdbConn,
    chapter_id: &str,
) -> BaseRest<()> {
    //
    diesel::update(t_chapter::table.find(chapter_id))
        .set((
            t_chapter::f_review_generation
                .eq(t_chapter::f_review_generation + 1),
            t_chapter::f_review_revision.eq(t_chapter::f_review_revision + 1),
        ))
        .execute(conn)
        .await
        .map_err(map_diesel)?;

    diesel::update(
        t_page_artwork::table
            .filter(t_page_artwork::f_chapter_id.eq(chapter_id)),
    )
    .set(
        t_page_artwork::f_issue_revision
            .eq(t_page_artwork::f_issue_revision + 1),
    )
    .execute(conn)
    .await
    .map_err(map_diesel)?;

    accept!(())
}
