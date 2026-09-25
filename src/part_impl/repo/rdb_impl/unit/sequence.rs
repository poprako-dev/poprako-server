//! RDB-backed Unit sequence reads and chain validation.

use std::collections::{HashMap, HashSet};

use diesel::expression::functions::declare_sql_function;
use diesel::pg::Pg;
use diesel::prelude::{
    ExpressionMethods as _, QueryDsl as _, SelectableHelper as _,
};
use diesel::sql_types::{Nullable, Text};
use diesel_async::RunQueryDsl as _;
use time::OffsetDateTime;
use tracing::instrument;

use poprako_rdb_core::RdbConn;

use crate::complex::unit::sequence as unit_sequence_complex;
use crate::model::read::proj::unit::{UnitInfo, UnitOrder};
use crate::part_impl::repo::rdb_impl::entity::unit::UnitInfoRow;
use crate::part_impl::repo::rdb_impl::numeric::usize_from_i32;
use crate::part_impl::repo::rdb_impl::schema::t_page::dsl::{
    f_chapter_id, f_index as page_index, t_page,
};
use crate::part_impl::repo::rdb_impl::schema::t_unit::dsl::{
    f_hidden_at, f_id as unit_id, f_next_id as unit_next_id, f_page_id,
    f_proofread_text, f_translated_text, t_unit,
};
use crate::result::{BaseError, BaseRest, accept};
use crate::shared::result::diesel;
use crate::value::unit::UnitTextPart;

// Number of Unit links consumed from one keyset query.
const UNIT_ORDER_QUERY_CHUNK_SIZE: i64 = 512;

// One extra row proves that another Unit-link chunk exists.
const UNIT_ORDER_QUERY_SENTINEL_LIMIT: i64 = UNIT_ORDER_QUERY_CHUNK_SIZE + 1;

#[declare_sql_function]
extern "SQL" {
    /// `PostgreSQL` literal substring position.
    // PostgreSQL literal substring position.
    fn strpos(
        string: Nullable<Text>,
        substring: Text,
    ) -> Nullable<diesel::sql_types::Integer>;
}

// One selected Unit with its adapter-only Page order.
struct RankedUnitInfo {
    // Adapter-only Page order.
    page_index: usize,

    // Selected Unit projection.
    unit_info: UnitInfo,
}

// Minimal persisted link needed to reconstruct one Unit chain.
struct UnitLink {
    // Permanent Unit identifier.
    id: String,

    // Linked-list successor.
    next_id: Option<String>,
}

/// Returns an unrecoverable error for a corrupt Unit chain.
pub fn corrupt_unit_chain_err() -> BaseError {
    //
    BaseError::Unrecoverable {
        message: "persisted Unit chain is corrupt".to_string(),
    }
}

/// Searches at most the requested number of visible matching Unit IDs.
pub async fn search_chapter_ids(
    conn: &mut RdbConn,
    chapter_id: &str,
    part: UnitTextPart,
    phrase: &str,
    fetch_count: usize,
) -> BaseRest<Vec<String>> {
    //
    let fetch_count = search_fetch_count(fetch_count)?;

    let query = t_unit
        .inner_join(t_page)
        .filter(f_chapter_id.eq(chapter_id))
        .filter(f_hidden_at.is_null())
        .select(unit_id)
        .into_boxed::<Pg>();

    let query = match part {
        //
        UnitTextPart::TranslatedText => {
            query.filter(strpos(f_translated_text, phrase).gt(0))
        }

        UnitTextPart::ProofreadText => {
            query.filter(strpos(f_proofread_text, phrase).gt(0))
        }
    };

    query
        .limit(fetch_count)
        .load::<String>(conn)
        .await
        .map_err(diesel)
}

/// Lists selected Units in Chapter Page and linked-list order.
pub async fn list_infos_in_chapter_order(
    conn: &mut RdbConn,
    ids: &[&str],
) -> BaseRest<Vec<UnitInfo>> {
    //
    if ids.is_empty() {
        return accept(Vec::new());
    }

    let rows = t_unit
        .inner_join(t_page)
        .filter(unit_id.eq_any(ids))
        .filter(f_hidden_at.is_null())
        .select((page_index, UnitInfoRow::as_select()))
        .load::<(i32, UnitInfoRow)>(conn)
        .await
        .map_err(diesel)?;

    if rows.len() != ids.len() {
        return Err(corrupt_unit_chain_err());
    }

    let mut candidates = rows
        .into_iter()
        .map(|(persisted_page_index, row)| {
            //
            Ok(RankedUnitInfo {
                page_index: usize_from_i32(
                    persisted_page_index,
                    "t_page.f_index",
                )?,
                unit_info: UnitInfo::from(row),
            })
        })
        .collect::<BaseRest<Vec<_>>>()?;

    let rank_by_unit_id = load_unit_ranks(conn, &candidates).await?;

    for candidate in &candidates {
        //
        if !rank_by_unit_id.contains_key(&candidate.unit_info.id) {
            return Err(corrupt_unit_chain_err());
        }
    }

    candidates.sort_by_key(|candidate| {
        //
        (
            candidate.page_index,
            rank_by_unit_id
                .get(&candidate.unit_info.id)
                .copied()
                .unwrap_or_default(),
        )
    });

    accept(
        candidates
            .into_iter()
            .map(|candidate| candidate.unit_info)
            .collect(),
    )
}

/// Lists Units for multiple Pages with one query and validates each linked list.
pub async fn list_infos_by_page_ids(
    conn: &mut RdbConn,
    page_ids: &[&str],
) -> BaseRest<Vec<UnitInfo>> {
    //
    let rows = t_unit
        .filter(f_page_id.eq_any(page_ids))
        .select(UnitInfoRow::as_select())
        .load::<UnitInfoRow>(conn)
        .await
        .map_err(diesel)?;

    let mut position_by_page_id = HashMap::with_capacity(page_ids.len());

    for (position, page_id) in page_ids.iter().enumerate() {
        position_by_page_id.entry(*page_id).or_insert(position);
    }

    let mut unit_infos_by_position =
        (0..page_ids.len()).map(|_| Vec::new()).collect::<Vec<_>>();

    for row in rows {
        //
        let unit_info = UnitInfo::from(row);

        let position = position_by_page_id
            .get(unit_info.page_id.as_str())
            .copied()
            .ok_or_else(corrupt_unit_chain_err)?;

        let page_unit_infos = unit_infos_by_position
            .get_mut(position)
            .ok_or_else(corrupt_unit_chain_err)?;

        page_unit_infos.push(unit_info);
    }

    let mut unit_infos = Vec::new();

    for mut page_unit_infos in unit_infos_by_position {
        //
        unit_sequence_complex::order_units(
            &mut page_unit_infos,
            |unit_info| unit_info.id.as_str(),
            |unit_info| unit_info.next_id.as_deref(),
        )?;

        unit_infos.extend(page_unit_infos);
    }

    accept(unit_infos)
}

/// Lists every requested persisted Unit that currently exists.
#[instrument(level = "info", skip_all)]
pub async fn list_infos_by_ids(
    conn: &mut RdbConn,
    ids: &[&str],
) -> BaseRest<Vec<UnitInfo>> {
    //
    let rows = t_unit
        .filter(unit_id.eq_any(ids))
        .select(UnitInfoRow::as_select())
        .load::<UnitInfoRow>(conn)
        .await
        .map_err(diesel)?;

    accept(rows.into_iter().map(UnitInfo::from).collect())
}

/// Lists the complete Unit chain, including tombstones.
#[instrument(level = "info", skip_all)]
pub async fn list_orders(
    conn: &mut RdbConn,
    page_id: &str,
) -> BaseRest<Vec<UnitOrder>> {
    //
    let mut rows =
        Vec::<(String, Option<String>, Option<OffsetDateTime>)>::new();

    loop {
        //
        let mut chunk =
            match rows.last().map(|(id, _, _)| id.as_str()) {
                //
                Some(after_id) => t_unit
                    .filter(f_page_id.eq(page_id))
                    .filter(unit_id.gt(after_id))
                    .select((unit_id, unit_next_id, f_hidden_at))
                    .order(unit_id.asc())
                    .limit(UNIT_ORDER_QUERY_SENTINEL_LIMIT)
                    .load::<(String, Option<String>, Option<OffsetDateTime>)>(
                        conn,
                    )
                    .await,

                None => t_unit
                    .filter(f_page_id.eq(page_id))
                    .select((unit_id, unit_next_id, f_hidden_at))
                    .order(unit_id.asc())
                    .limit(UNIT_ORDER_QUERY_SENTINEL_LIMIT)
                    .load::<(String, Option<String>, Option<OffsetDateTime>)>(
                        conn,
                    )
                    .await,
            }
            .map_err(diesel)?;

        let has_next_chunk =
            i64::try_from(chunk.len()).is_ok_and(|chunk_len| {
                chunk_len == UNIT_ORDER_QUERY_SENTINEL_LIMIT
            });

        if has_next_chunk {
            let _ = chunk.pop();
        }

        if has_next_chunk && chunk.is_empty() {
            return Err(corrupt_unit_chain_err());
        }

        rows.extend(chunk);

        if !has_next_chunk {
            break;
        }
    }

    let mut orders = rows
        .into_iter()
        .map(|(id, next_id, hidden_at)| UnitOrder {
            id,
            next_id,
            is_hidden: hidden_at.is_some(),
        })
        .collect::<Vec<_>>();

    unit_sequence_complex::order_units(
        &mut orders,
        |order| order.id.as_str(),
        |order| order.next_id.as_deref(),
    )?;

    accept(orders)
}

// Converts a Unit search fetch count into its SQL LIMIT representation.
fn search_fetch_count(fetch_count: usize) -> BaseRest<i64> {
    //
    i64::try_from(fetch_count).map_err(|_| {
        //
        tracing::error!(
            fetch_count,
            "unrecoverable error: Unit search fetch count exceeds BIGINT"
        );

        BaseError::Unrecoverable {
            message: "Unit search fetch count exceeds BIGINT".into(),
        }
    })
}

// Loads and validates complete Unit links for the Pages containing matches.
async fn load_unit_ranks(
    conn: &mut RdbConn,
    candidates: &[RankedUnitInfo],
) -> BaseRest<HashMap<String, usize>> {
    //
    let mut page_ids = candidates
        .iter()
        .map(|candidate| candidate.unit_info.page_id.as_str())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    page_ids.sort_unstable();

    let rows = t_unit
        .filter(f_page_id.eq_any(&page_ids))
        .select((f_page_id, unit_id, unit_next_id))
        .load::<(String, String, Option<String>)>(conn)
        .await
        .map_err(diesel)?;

    let mut links_by_page_id = HashMap::<String, Vec<UnitLink>>::new();

    for (page_id, id, next_id) in rows {
        //
        links_by_page_id
            .entry(page_id)
            .or_default()
            .push(UnitLink { id, next_id });
    }

    let mut rank_by_unit_id = HashMap::new();

    for page_id in page_ids {
        //
        let mut links = links_by_page_id.remove(page_id).unwrap_or_default();

        unit_sequence_complex::order_units(
            &mut links,
            |link| link.id.as_str(),
            |link| link.next_id.as_deref(),
        )?;

        for (rank, link) in links.into_iter().enumerate() {
            rank_by_unit_id.insert(link.id, rank);
        }
    }

    accept(rank_by_unit_id)
}
