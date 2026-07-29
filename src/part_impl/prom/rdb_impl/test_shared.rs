use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::part_impl::repo::rdb_impl::schema::t_local_message;
use crate::part_impl::shared::RdbCore;
use crate::part_impl::shared::result::diesel as diesel_error;
use crate::result::{BaseResult, accept};

pub async fn reset(shared: &RdbCore, prefix: &str) {
    //
    cleanup(shared, prefix).await.unwrap();

    assert_no_leftovers(shared, prefix).await.unwrap();
}

pub async fn cleanup(shared: &RdbCore, prefix: &str) -> BaseResult<()> {
    //
    let mut conn = shared.get().await?;

    let id_pattern = format!("{}%", prefix);

    diesel::delete(
        t_local_message::table.filter(t_local_message::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    accept(())
}

pub async fn assert_no_leftovers(
    shared: &RdbCore,
    prefix: &str,
) -> BaseResult<()> {
    //
    let mut conn = shared.get().await?;

    let id_pattern = format!("{}%", prefix);

    let local_message_count: i64 = t_local_message::table
        .filter(t_local_message::f_id.like(&id_pattern))
        .count()
        .get_result(&mut conn)
        .await
        .map_err(diesel_error)?;

    assert_eq!(local_message_count, 0);

    accept(())
}
