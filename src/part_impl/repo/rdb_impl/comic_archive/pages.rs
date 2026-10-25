//! Locked source page and Unit snapshots for comic archives.

use std::collections::HashMap;

use diesel::prelude::{
    ExpressionMethods as _, QueryDsl as _, SelectableHelper as _,
};
use diesel_async::RunQueryDsl as _;

use poprako_rdb_core::RdbConn;

use crate::model::read::proj::comic_archive::ComicArchivePageSourceSnapshot;
use crate::model::read::proj::page_source::PageSourceInfo;
use crate::model::read::proj::unit::UnitInfo;
use crate::part_impl::repo::rdb_impl::entity::page_source::PageSourceInfoRecord;
use crate::part_impl::repo::rdb_impl::entity::unit::UnitInfoRecord;
use crate::part_impl::repo::rdb_impl::schema::t_page_source::dsl::{
    f_chapter_id as page_source_chapter_id, f_id as page_source_id,
    f_index as page_source_index, t_page_source,
};
use crate::part_impl::repo::rdb_impl::schema::t_unit::dsl::{
    f_page_source_id as unit_page_source_id, t_unit,
};
use crate::result::{BaseError, BaseRest};
use crate::shared::result::diesel;

/// Locks ordered source pages with their translation Units.
pub async fn load_archive_page_sources(
    conn: &mut RdbConn,
    source_chapter_ids: &[String],
) -> BaseRest<(Vec<PageSourceInfo>, Vec<UnitInfo>)> {
    //
    let page_source_records = t_page_source
        .filter(page_source_chapter_id.eq_any(source_chapter_ids))
        .select(PageSourceInfoRecord::as_select())
        .order_by((
            page_source_chapter_id.asc(),
            page_source_index.asc(),
            page_source_id.asc(),
        ))
        .for_update()
        .load::<PageSourceInfoRecord>(conn)
        .await
        .map_err(diesel)?;

    let page_source_infos = page_source_records
        .into_iter()
        .map(TryInto::try_into)
        .collect::<BaseRest<Vec<PageSourceInfo>>>()?;

    let source_page_source_ids = page_source_infos
        .iter()
        .map(|page_source_info| page_source_info.id.clone())
        .collect::<Vec<_>>();

    let unit_records = t_unit
        .filter(unit_page_source_id.eq_any(&source_page_source_ids))
        .select(UnitInfoRecord::as_select())
        .order_by(unit_page_source_id.asc())
        .for_update()
        .load::<UnitInfoRecord>(conn)
        .await
        .map_err(diesel)?;

    let unit_infos = unit_records
        .into_iter()
        .map(Into::into)
        .collect::<Vec<UnitInfo>>();

    accept!((page_source_infos, unit_infos))
}

/// Assembles ordered source page details after all descendant records have been locked.
pub fn assemble_page_source_snapshots(
    page_source_infos: Vec<PageSourceInfo>,
    unit_infos: Vec<UnitInfo>,
) -> BaseRest<HashMap<String, Vec<ComicArchivePageSourceSnapshot>>> {
    //
    let mut unit_infos_by_page = HashMap::new();

    for unit_info in unit_infos {
        //
        unit_infos_by_page
            .entry(unit_info.page_source_id.clone())
            .or_insert_with(Vec::new)
            .push(unit_info);
    }

    let mut page_source_snapshots_by_chapter = HashMap::new();

    for page_source_info in page_source_infos {
        //
        let unordered_unit_infos = unit_infos_by_page
            .remove(&page_source_info.id)
            .unwrap_or_default();

        let mut unit_infos = order_unit_infos(unordered_unit_infos)?;

        unit_infos.retain(|unit_info| unit_info.hidden_at.is_none());

        page_source_snapshots_by_chapter
            .entry(page_source_info.chapter_id.clone())
            .or_insert_with(Vec::new)
            .push(ComicArchivePageSourceSnapshot {
                page_source_info,
                unit_infos,
            });
    }

    accept!(page_source_snapshots_by_chapter)
}

// Reorder chained unit infos by next_id links and return only visible units.
fn order_unit_infos(unit_infos: Vec<UnitInfo>) -> BaseRest<Vec<UnitInfo>> {
    //
    if unit_infos.is_empty() {
        return accept!(Vec::new());
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
            return Err(corrupt_unit_chain_error());
        }

        let Some(predecessor_count) = predecessor_counts.get_mut(next_id)
        else {
            return Err(corrupt_unit_chain_error());
        };

        *predecessor_count += 1;

        if *predecessor_count > 1 {
            return Err(corrupt_unit_chain_error());
        }
    }

    let head_ids = predecessor_counts
        .iter()
        .filter_map(|(id, count)| (*count == 0).then_some(id.as_str()))
        .collect::<Vec<_>>();

    let [head_id] = head_ids.as_slice() else {
        return Err(corrupt_unit_chain_error());
    };

    let (mut current_id, mut visible_infos) = (
        Some((*head_id).to_string()),
        Vec::with_capacity(infos_by_id.len()),
    );

    while let Some(id) = current_id.as_ref() {
        //
        let Some(unit_info) = infos_by_id.remove(id) else {
            return Err(corrupt_unit_chain_error());
        };

        current_id.clone_from(&unit_info.next_id);

        if unit_info.hidden_at.is_none() {
            visible_infos.push(unit_info);
        }
    }

    if !infos_by_id.is_empty() {
        return Err(corrupt_unit_chain_error());
    }

    accept!(visible_infos)
}

// Standardize chain-corruption failures for unit graph validation.
fn corrupt_unit_chain_error() -> BaseError {
    //
    unrecoverable_error! {
        message: "persisted Unit chain is corrupt".to_string();
        "persisted Unit chain is corrupt",
    }
}
