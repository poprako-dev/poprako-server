use diesel::prelude::{
    ExpressionMethods as _, QueryDsl as _, TextExpressionMethods as _,
};
use diesel_async::RunQueryDsl as _;
use time::{Duration, OffsetDateTime};

use poprako_rdb_core::RdbCore;

use super::rdb_obj_prom_rdb_impl::claim_task;
use crate::part_impl::repo::rdb_impl::schema::t_obj_prom_task;

const PREFIX: &str = "rdb-test-obj-claim-";
const PENDING: &str = "obj_prom_status:pending";
const OPERATOR: &str = "obj_prom_status:operator";

pub async fn concurrent_claim_is_unique_ordered_and_overflow_safe(
    shared: RdbCore,
) {
    cleanup(&shared).await;

    let now = OffsetDateTime::now_utc();

    insert_task(&shared, "oldest", now - Duration::seconds(2), 0).await;

    let first_core = shared.clone();
    let second_core = shared.clone();

    let first_claim =
        tokio::spawn(async move { claim_task(&first_core).await });
    let second_claim =
        tokio::spawn(async move { claim_task(&second_core).await });

    let first_task = first_claim.await.unwrap().unwrap();
    let second_task = second_claim.await.unwrap().unwrap();

    let claimed_tasks = [first_task, second_task]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    assert_eq!(claimed_tasks.len(), 1);

    let Some(claimed_task) = claimed_tasks.as_slice().first() else {
        panic!("one task must be claimed");
    };

    assert!(claimed_task.id.ends_with("oldest"));
    assert_eq!(claimed_task.lease, 1);

    insert_task(&shared, "later", now, 4).await;

    let later_task = claim_task(&shared).await.unwrap().unwrap();

    assert!(later_task.id.ends_with("later"));
    assert_eq!(later_task.lease, 5);

    insert_task(&shared, "overflow", now, i64::MAX).await;

    assert!(claim_task(&shared).await.unwrap().is_none());

    let mut conn = shared.get().await.unwrap();

    let overflow_status = t_obj_prom_task::table
        .filter(t_obj_prom_task::f_id.eq(format!("{}overflow", PREFIX)))
        .select((t_obj_prom_task::f_status, t_obj_prom_task::f_error))
        .first::<(String, Option<String>)>(&mut conn)
        .await
        .unwrap();

    assert_eq!(overflow_status.0, OPERATOR);
    assert_eq!(
        overflow_status.1.as_deref(),
        Some("object task lease overflow")
    );

    cleanup(&shared).await;
}

async fn insert_task(
    shared: &RdbCore,
    suffix: &str,
    created_at: OffsetDateTime,
    lease: i64,
) {
    let mut conn = shared.get().await.unwrap();

    diesel::insert_into(t_obj_prom_task::table)
        .values((
            t_obj_prom_task::f_id.eq(format!("{}{}", PREFIX, suffix)),
            t_obj_prom_task::f_topic.eq("page_image"),
            t_obj_prom_task::f_oper.eq("obj_prom_oper:check"),
            t_obj_prom_task::f_obj_id.eq(format!("{}page", PREFIX)),
            t_obj_prom_task::f_version.eq(1_i64),
            t_obj_prom_task::f_key.eq(format!("{}key", PREFIX)),
            t_obj_prom_task::f_generation.eq(0_i64),
            t_obj_prom_task::f_status.eq(PENDING),
            t_obj_prom_task::f_visible_at.eq(created_at),
            t_obj_prom_task::f_retried_count.eq(0_i64),
            t_obj_prom_task::f_lease.eq(lease),
            t_obj_prom_task::f_error.eq(None::<String>),
            t_obj_prom_task::f_created_at.eq(created_at),
            t_obj_prom_task::f_updated_at.eq(created_at),
        ))
        .execute(&mut conn)
        .await
        .unwrap();
}

async fn cleanup(shared: &RdbCore) {
    let mut conn = shared.get().await.unwrap();

    diesel::delete(
        t_obj_prom_task::table
            .filter(t_obj_prom_task::f_id.like(format!("{}%", PREFIX))),
    )
    .execute(&mut conn)
    .await
    .unwrap();

    let remaining = t_obj_prom_task::table
        .filter(t_obj_prom_task::f_id.like(format!("{}%", PREFIX)))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .unwrap();

    assert_eq!(remaining, 0);
}
