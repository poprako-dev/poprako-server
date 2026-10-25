//! Byte-bounded permanent comic archive payload queries.

use diesel::define_sql_function;
use diesel::expression::BoxableExpression;
use diesel::pg::Pg;
use diesel::prelude::{
    BoolExpressionMethods as _, ExpressionMethods as _, QueryDsl as _,
};
use diesel::sql_types::{Bool, Text};
use diesel_async::RunQueryDsl as _;
use time::OffsetDateTime;
use tracing::instrument;

use poprako_rdb_core::RdbConn;

use crate::model::read::proj::comic_archive::ComicArchivePayload;
use crate::part::repo::oper::comic_archive::ListComicArchivePayloads;
use crate::part_impl::repo::rdb_impl::schema::t_comic_archive::table as ComicArchiveTable;
use crate::result::{BaseError, BaseRest};
use crate::shared::result::diesel;

define_sql_function! {
    /// Measures stored UTF-8 bytes without transferring the payload.
    fn octet_length(value: Text) -> Integer;
}

/// Loads one deterministic prefix bounded by metadata count and payload bytes.
#[instrument(level = "info", skip_all)]
pub async fn list_payloads(
    conn: &mut RdbConn,
    oper: &ListComicArchivePayloads<'_>,
) -> BaseRest<Vec<ComicArchivePayload>> {
    //
    use crate::part_impl::repo::rdb_impl::schema::t_comic_archive::dsl::{
        f_archived_payload, f_created_at, f_id, f_team_id, t_comic_archive,
    };

    let mut query = t_comic_archive
        .filter(f_team_id.eq(oper.team_id))
        .into_boxed();

    let mut intervals = Box::new(
        f_created_at
            .ge(OffsetDateTime::UNIX_EPOCH)
            .and(f_created_at.lt(OffsetDateTime::UNIX_EPOCH)),
    )
        as Box<dyn BoxableExpression<ComicArchiveTable, Pg, SqlType = Bool>>;

    for month in oper.months {
        //
        intervals =
            Box::new(intervals.or(
                f_created_at.ge(month.start).and(f_created_at.lt(month.end)),
            ));
    }

    query = query.filter(intervals);

    if let Some((created_at, id)) = oper.after {
        //
        query = query.filter(
            f_created_at
                .gt(created_at)
                .or(f_created_at.eq(created_at).and(f_id.gt(id))),
        );
    }

    let metadata = query
        .select((f_id, octet_length(f_archived_payload)))
        .order_by((f_created_at.asc(), f_id.asc()))
        .limit(64)
        .load::<(String, i32)>(conn)
        .await
        .map_err(diesel)?;

    let mut bytes = 0_u64;

    let mut ids = Vec::new();

    for (id, size) in metadata {
        //
        let size = u64::try_from(size).map_err(|_| {
            //
            unrecoverable_error! {
                message: "archive payload has an invalid byte length".into();
                "archive payload length invariant failed",
            }
        })?;

        if size > oper.payload_budget_bytes {
            //
            if !ids.is_empty() {
                break;
            }

            tracing::warn!(
                operation = "export_comic_archives",
                "archive payload exceeds export memory budget"
            );

            return Err(BaseError::Unavailable {
                message: "archive export memory budget exceeded".into(),
            });
        }

        if bytes + size > oper.payload_budget_bytes {
            break;
        }

        bytes += size;

        ids.push(id);
    }

    if ids.is_empty() {
        return accept!(Vec::new());
    }

    let records = t_comic_archive
        .filter(f_team_id.eq(oper.team_id))
        .filter(f_id.eq_any(ids))
        .select((f_id, f_created_at, f_archived_payload))
        .order_by((f_created_at.asc(), f_id.asc()))
        .load::<(String, OffsetDateTime, String)>(conn)
        .await
        .map_err(diesel)?;

    accept!(
        records
            .into_iter()
            .map(|(id, created_at, payload)| ComicArchivePayload {
                id,
                payload,
                created_at,
            })
            .collect()
    )
}
