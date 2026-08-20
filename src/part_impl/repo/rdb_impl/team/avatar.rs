//! Team avatar persistence boundary.

use crate::model::write::team::TeamAvatarReservation;
use crate::part_impl::repo::rdb_impl::team::info;
use crate::result::BaseRest;
use crate::shared::RdbConn;
use crate::value::image::{ImageExt, ImageHash};

/// Reserve a team avatar key and version.
pub async fn reserve_avatar(
    conn: &mut RdbConn,
    id: &str,
    image_hash: &ImageHash,
    image_ext: ImageExt,
) -> BaseRest<TeamAvatarReservation> {
    info::reserve_avatar(conn, id, image_hash, image_ext).await
}

/// Mark a reserved team avatar as uploaded.
pub async fn mark_avatar_uploaded(
    conn: &mut RdbConn,
    id: &str,
    version: u32,
    avatar_key: Option<&str>,
    avatar_uploaded: bool,
) -> BaseRest<()> {
    //
    info::mark_avatar_uploaded(conn, id, version, avatar_key, avatar_uploaded)
        .await
}
