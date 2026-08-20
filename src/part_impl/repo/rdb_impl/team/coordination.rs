//! Team row locking and workset-index allocation boundary.

use crate::part_impl::repo::rdb_impl::team::info;
use crate::result::BaseRest;
use crate::shared::RdbConn;

/// Lock a team row for coordinated mutation.
pub async fn lock_team(conn: &mut RdbConn, id: &str) -> BaseRest<()> {
    info::lock_team(conn, id).await
}

/// Allocate the next team workset index.
pub async fn increment_workset_next_index(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<i32> {
    info::increment_workset_next_index(conn, id).await
}
