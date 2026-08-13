//! Permanent comic archive payload queries.

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;
use tracing::instrument;

use crate::result::{BaseRest, accept};
use crate::shared::RdbConn;
use crate::shared::result::diesel;
use crate::value::comic_archive::ComicArchiveMonth;

// Load archive payloads by month window and return timestamped serialized blobs.
#[instrument(level = "info", skip_all)]
/// Lists serialized archive payloads in the requested months.
pub async fn list_payloads(
    conn: &mut RdbConn,
    team_id: &str,
    months: &[ComicArchiveMonth],
) -> BaseRest<Vec<(OffsetDateTime, String)>> {
    // Queryable projection row for one archive slot.
    #[derive(Queryable)]
    struct ArchivePayloadRow {
        // UTC timestamp when the archive slot was created.
        created_at: OffsetDateTime,
        // Serialized payload snapshot JSON for a retention slot.
        payload: String,
    }

    use crate::part_impl::repo::rdb_impl::schema::t_comic_archive::dsl::{
        f_archived_payload, f_created_at, f_team_id, t_comic_archive,
    };

    let Some(first_month) = months.first() else {
        return accept(Vec::new());
    };

    let Some(last_month) = months.last() else {
        return accept(Vec::new());
    };

    let query = t_comic_archive
        .filter(f_team_id.eq(team_id))
        .filter(f_created_at.ge(first_month.start))
        .filter(f_created_at.lt(last_month.end))
        .select((f_created_at, f_archived_payload))
        .into_boxed();

    let rows = query
        .order_by(f_created_at.asc())
        .load::<ArchivePayloadRow>(conn)
        .await
        .map_err(diesel)?;

    accept(
        rows.into_iter()
            .filter(|row| {
                //
                months.iter().any(|month| {
                    row.created_at >= month.start && row.created_at < month.end
                })
            })
            .map(|row| (row.created_at, row.payload))
            .collect(),
    )
}
