//! Locked independent composite pages for immutable archives.

use std::collections::HashMap;

use diesel::prelude::{
    ExpressionMethods as _, QueryDsl as _, SelectableHelper as _,
};
use diesel_async::RunQueryDsl as _;

use poprako_rdb_core::RdbConn;

use crate::model::read::proj::comic_archive::ComicArchivePageArtworkSnapshot;
use crate::model::read::proj::issue::IssueInfo;
use crate::model::read::proj::page_artwork::PageArtworkInfo;
use crate::part_impl::repo::rdb_impl::entity::issue::IssueRow;
use crate::part_impl::repo::rdb_impl::entity::page_artwork::PageArtworkRow;
use crate::part_impl::repo::rdb_impl::schema::{t_issue, t_page_artwork};
use crate::result::{BaseRest, accept};
use crate::shared::result::diesel;

/// Loads composites even when the Chapter has no source Pages.
pub async fn load_archive_page_artworks(
    conn: &mut RdbConn,
    chapter_ids: &[String],
) -> BaseRest<HashMap<String, Vec<ComicArchivePageArtworkSnapshot>>> {
    //
    let rows = t_page_artwork::table
        .filter(t_page_artwork::f_chapter_id.eq_any(chapter_ids))
        .order((
            t_page_artwork::f_chapter_id.asc(),
            t_page_artwork::f_index.asc(),
        ))
        .select(PageArtworkRow::as_select())
        .for_update()
        .load::<PageArtworkRow>(conn)
        .await
        .map_err(diesel)?;

    let page_artwork_infos = rows
        .into_iter()
        .map(TryInto::try_into)
        .collect::<BaseRest<Vec<PageArtworkInfo>>>()?;

    let page_artwork_ids = page_artwork_infos
        .iter()
        .map(|page_artwork_info| page_artwork_info.id.as_str())
        .collect::<Vec<_>>();

    let rows = t_issue::table
        .filter(t_issue::f_page_artwork_id.eq_any(&page_artwork_ids))
        .order(t_issue::f_index.asc())
        .select(IssueRow::as_select())
        .for_update()
        .load::<IssueRow>(conn)
        .await
        .map_err(diesel)?;

    let issue_infos = rows
        .into_iter()
        .map(TryInto::try_into)
        .collect::<BaseRest<Vec<IssueInfo>>>()?;

    let mut issue_infos_by_page_artwork =
        HashMap::<String, Vec<IssueInfo>>::new();

    for issue_info in issue_infos {
        //
        issue_infos_by_page_artwork
            .entry(issue_info.page_artwork_id.clone())
            .or_default()
            .push(issue_info);
    }

    let mut page_artwork_snapshots_by_chapter =
        HashMap::<String, Vec<ComicArchivePageArtworkSnapshot>>::new();

    for page_artwork_info in page_artwork_infos {
        //
        let issue_infos = issue_infos_by_page_artwork
            .remove(&page_artwork_info.id)
            .unwrap_or_default();

        page_artwork_snapshots_by_chapter
            .entry(page_artwork_info.chapter_id.clone())
            .or_default()
            .push(ComicArchivePageArtworkSnapshot {
                page_artwork_info,
                issue_infos,
            });
    }

    accept(page_artwork_snapshots_by_chapter)
}
