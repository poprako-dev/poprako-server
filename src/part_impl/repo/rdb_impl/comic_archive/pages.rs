//! Locked Page, Unit and current review snapshots for comic archives.

use std::collections::HashMap;

use diesel::prelude::{
    ExpressionMethods as _, QueryDsl as _, SelectableHelper as _,
};
use diesel_async::RunQueryDsl as _;

use poprako_rdb_core::RdbConn;

use crate::model::read::proj::comic_archive::ComicArchivePageSnapshot;
use crate::model::read::proj::issue::IssueInfo;
use crate::model::read::proj::page::PageInfo;
use crate::model::read::proj::unit::UnitInfo;
use crate::part_impl::repo::rdb_impl::entity::issue::IssueRow;
use crate::part_impl::repo::rdb_impl::entity::page::PageInfoRow;
use crate::part_impl::repo::rdb_impl::entity::unit::UnitInfoRow;
use crate::part_impl::repo::rdb_impl::schema::t_issue;
use crate::part_impl::repo::rdb_impl::schema::t_page::dsl::{
    f_chapter_id as page_chapter_id, f_id as page_id, f_index as page_index,
    t_page,
};
use crate::part_impl::repo::rdb_impl::schema::t_unit::dsl::{
    f_page_id as unit_page_id, t_unit,
};
use crate::result::{BaseError, BaseRest, accept};
use crate::shared::result::diesel;

/// Locks ordered Pages with their Units and current review details.
pub async fn load_archive_pages(
    conn: &mut RdbConn,
    source_chapter_ids: &[String],
) -> BaseRest<(Vec<PageInfo>, Vec<UnitInfo>, Vec<IssueInfo>)> {
    //
    let page_rows = t_page
        .filter(page_chapter_id.eq_any(source_chapter_ids))
        .select(PageInfoRow::as_select())
        .order_by((page_chapter_id.asc(), page_index.asc(), page_id.asc()))
        .for_update()
        .load::<PageInfoRow>(conn)
        .await
        .map_err(diesel)?;

    let page_infos = page_rows
        .into_iter()
        .map(TryInto::try_into)
        .collect::<BaseRest<Vec<PageInfo>>>()?;

    let source_page_ids = page_infos
        .iter()
        .map(|page_info| page_info.id.clone())
        .collect::<Vec<_>>();

    let unit_rows = t_unit
        .filter(unit_page_id.eq_any(&source_page_ids))
        .select(UnitInfoRow::as_select())
        .order_by(unit_page_id.asc())
        .for_update()
        .load::<UnitInfoRow>(conn)
        .await
        .map_err(diesel)?;

    let unit_infos = unit_rows
        .into_iter()
        .map(Into::into)
        .collect::<Vec<UnitInfo>>();

    let issue_rows = t_issue::table
        .filter(t_issue::f_page_id.eq_any(&source_page_ids))
        .select(IssueRow::as_select())
        .order((t_issue::f_page_id.asc(), t_issue::f_index.asc()))
        .for_update()
        .load::<IssueRow>(conn)
        .await
        .map_err(diesel)?;

    let issue_infos = issue_rows
        .into_iter()
        .map(TryInto::try_into)
        .collect::<BaseRest<Vec<_>>>()?;

    accept((page_infos, unit_infos, issue_infos))
}

/// Assembles ordered Page details after all descendant rows have been locked.
pub fn assemble_page_snapshots(
    page_infos: Vec<PageInfo>,
    unit_infos: Vec<UnitInfo>,
    issue_infos: Vec<IssueInfo>,
) -> BaseRest<HashMap<String, Vec<ComicArchivePageSnapshot>>> {
    //
    let mut unit_infos_by_page = HashMap::new();

    for unit_info in unit_infos {
        //
        unit_infos_by_page
            .entry(unit_info.page_id.clone())
            .or_insert_with(Vec::new)
            .push(unit_info);
    }

    let mut issue_infos_by_page = HashMap::new();

    for issue_info in issue_infos {
        //
        issue_infos_by_page
            .entry(issue_info.page_id.clone())
            .or_insert_with(Vec::new)
            .push(issue_info);
    }

    let mut page_snapshots_by_chapter = HashMap::new();

    for page_info in page_infos {
        //
        let unordered_unit_infos =
            unit_infos_by_page.remove(&page_info.id).unwrap_or_default();

        let mut unit_infos = order_unit_infos(unordered_unit_infos)?;

        unit_infos.retain(|unit_info| unit_info.hidden_at.is_none());

        page_snapshots_by_chapter
            .entry(page_info.chapter_id.clone())
            .or_insert_with(Vec::new)
            .push(ComicArchivePageSnapshot {
                issue_infos: issue_infos_by_page
                    .remove(&page_info.id)
                    .unwrap_or_default(),
                page_info,
                unit_infos,
            });
    }

    accept(page_snapshots_by_chapter)
}

// Reorder chained unit infos by next_id links and return only visible units.
fn order_unit_infos(unit_infos: Vec<UnitInfo>) -> BaseRest<Vec<UnitInfo>> {
    //
    if unit_infos.is_empty() {
        return accept(Vec::new());
    }

    let mut infos_by_id = unit_infos
        .into_iter()
        .map(|unit_info| (unit_info.id.clone(), unit_info))
        .collect::<HashMap<_, _>>();

    let mut predecessor_counts = infos_by_id
        .keys()
        .map(|id| (id.clone(), 0_usize))
        .collect::<HashMap<_, _>>();

    for unit_info in infos_by_id.values() {
        //
        let Some(next_id) = unit_info.next_id.as_ref() else {
            continue;
        };

        if next_id == &unit_info.id {
            return Err(corrupt_unit_chain_err());
        }

        let Some(predecessor_count) = predecessor_counts.get_mut(next_id)
        else {
            return Err(corrupt_unit_chain_err());
        };

        *predecessor_count += 1;

        if *predecessor_count > 1 {
            return Err(corrupt_unit_chain_err());
        }
    }

    let head_ids = predecessor_counts
        .iter()
        .filter_map(|(id, count)| (*count == 0).then_some(id.as_str()))
        .collect::<Vec<_>>();

    let [head_id] = head_ids.as_slice() else {
        return Err(corrupt_unit_chain_err());
    };

    let (mut current_id, mut visible_infos) = (
        Some((*head_id).to_string()),
        Vec::with_capacity(infos_by_id.len()),
    );

    while let Some(id) = current_id.as_ref() {
        //
        let Some(unit_info) = infos_by_id.remove(id) else {
            return Err(corrupt_unit_chain_err());
        };

        current_id.clone_from(&unit_info.next_id);

        if unit_info.hidden_at.is_none() {
            visible_infos.push(unit_info);
        }
    }

    if !infos_by_id.is_empty() {
        return Err(corrupt_unit_chain_err());
    }

    accept(visible_infos)
}

// Standardize chain-corruption failures for unit graph validation.
fn corrupt_unit_chain_err() -> BaseError {
    BaseError::Unrecoverable {
        msg: "persisted Unit chain is corrupt".to_string(),
    }
}
