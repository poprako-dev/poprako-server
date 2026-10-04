use diesel::{ExpressionMethods as _, QueryDsl as _};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{Nucl as _, OperStep as _};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use poprako_prom::general::rdb_impl::LocalTaskStatus;

use crate::part::nucl::Serial;
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::prom::rdb_impl::entity::LocalTaskEntryRow;
use crate::part_impl::prom::rdb_impl::repo::{
    ClaimPending, PurgeDead, RdbPromRepo, ResetStuck,
};
use crate::part_impl::repo::rdb_impl::schema::t_local_message;
use crate::result::BaseError;
use crate::shared::test_rdb::start;

diesel::table! {
    pg_catalog.pg_stat_xact_user_tables (relid) {
        relid -> Oid,
        relname -> Text,
        seq_tup_read -> BigInt,
        idx_tup_fetch -> BigInt,
    }
}

// claim_reads_are_bounded(ClaimPending)(negative): an unrelated topic's backlog cannot add heap reads to this topic's claim.
#[tokio::test]
#[serial_test::serial(prom_rdb)]
async fn claim_reads_are_bounded() {
    //
    let test_rdb = start().await;

    let core = test_rdb.core();

    let mut conn = core.get().await.unwrap();

    let now = OffsetDateTime::now_utc();

    let ids = (0..8192)
        .map(|index| format!("prom-cost-{:05}", index))
        .collect::<Vec<_>>();

    let entries = ids
        .iter()
        .enumerate()
        .map(|(index, id)| LocalTaskEntryRow {
            f_id: id,
            f_topic: match index {
                //
                0 => "chapter",

                _ => "invitation",
            },
            f_status: LocalTaskStatus::Pending,
            f_claim_token: None,
            f_payload: serde_json::json!({}),
            f_visible_at: now - Duration::minutes(1),
            f_created_at: now - Duration::minutes(1),
            f_updated_at: now,
        })
        .collect::<Vec<_>>();

    diesel::insert_into(t_local_message::table)
        .values(&entries)
        .execute(&mut conn)
        .await
        .unwrap();

    drop(conn);

    let nucl = RdbNucl::<Serial>::new(core.clone());

    let repo = RdbPromRepo::new();

    let reads = nucl
        .coord(async |context| {
            //
            let before = pg_stat_xact_user_tables::table
                .filter(pg_stat_xact_user_tables::relname.eq("t_local_message"))
                .select(
                    pg_stat_xact_user_tables::seq_tup_read
                        + pg_stat_xact_user_tables::idx_tup_fetch,
                )
                .first::<i64>(context.conn())
                .await
                .unwrap();

            let row = ClaimPending::new("chapter")
                .step_on(&repo, context)
                .await?
                .unwrap();

            assert_eq!(row.topic(), "chapter");

            assert_eq!(row.id(), "prom-cost-00000");

            let after = pg_stat_xact_user_tables::table
                .filter(pg_stat_xact_user_tables::relname.eq("t_local_message"))
                .select(
                    pg_stat_xact_user_tables::seq_tup_read
                        + pg_stat_xact_user_tables::idx_tup_fetch,
                )
                .first::<i64>(context.conn())
                .await
                .unwrap();

            Ok::<_, BaseError>(after - before)
        })
        .await
        .unwrap();

    assert!(
        reads < 32,
        "claiming chapter fetched {reads} heap tuples with 8191 unrelated messages"
    );

    let mut conn = core.get().await.unwrap();

    let pending_invitations = t_local_message::table
        .filter(t_local_message::f_topic.eq("invitation"))
        .filter(t_local_message::f_status.eq(LocalTaskStatus::Pending.as_str()))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .unwrap();

    assert_eq!(pending_invitations, 8191);

    let missing = nucl
        .coord(async |context| {
            ClaimPending::new("missing").step_on(&repo, context).await
        })
        .await
        .unwrap();

    assert!(missing.is_none());
}

// maintenance_preserves_other_topics(ResetStuck/PurgeDead)(negative): recovery and retention changes remain within the requested queue.
#[tokio::test]
#[serial_test::serial(prom_rdb)]
async fn maintenance_preserves_other_topics() {
    //
    let test_rdb = start().await;

    let core = test_rdb.core();

    let mut conn = core.get().await.unwrap();

    let now = OffsetDateTime::now_utc();

    let entries = [
        ("chapter-processing", "chapter", LocalTaskStatus::Processing),
        (
            "invitation-processing",
            "invitation",
            LocalTaskStatus::Processing,
        ),
        ("chapter-dead", "chapter", LocalTaskStatus::Dead),
        ("invitation-dead", "invitation", LocalTaskStatus::Dead),
    ]
    .map(|(id, topic, status)| LocalTaskEntryRow {
        f_id: id,
        f_topic: topic,
        f_status: status,
        f_claim_token: matches!(status, LocalTaskStatus::Processing)
            .then_some(Uuid::from_u128(1)),
        f_payload: serde_json::json!({}),
        f_visible_at: now - Duration::days(31),
        f_created_at: now - Duration::days(31),
        f_updated_at: now - Duration::days(31),
    });

    diesel::insert_into(t_local_message::table)
        .values(&entries)
        .execute(&mut conn)
        .await
        .unwrap();

    let nucl = RdbNucl::<Serial>::new(core.clone());

    let repo = RdbPromRepo::new();

    let cutoff = now - Duration::days(30);

    let purged = nucl
        .coord(async |context| {
            //
            ResetStuck::new("chapter", &cutoff)
                .step_on(&repo, context)
                .await?;

            PurgeDead::new("chapter", &cutoff)
                .step_on(&repo, context)
                .await
        })
        .await
        .unwrap();

    assert_eq!(purged, 1);

    let chapter = t_local_message::table
        .filter(t_local_message::f_id.eq("chapter-processing"))
        .select((
            t_local_message::f_status,
            t_local_message::f_claim_token,
            t_local_message::f_retried_count,
        ))
        .first::<(String, Option<Uuid>, i64)>(&mut conn)
        .await
        .unwrap();

    assert_eq!(chapter, (LocalTaskStatus::Pending.as_str().into(), None, 1));

    let invitations = t_local_message::table
        .filter(t_local_message::f_topic.eq("invitation"))
        .order_by(t_local_message::f_id)
        .select((
            t_local_message::f_id,
            t_local_message::f_status,
            t_local_message::f_claim_token,
            t_local_message::f_retried_count,
        ))
        .load::<(String, String, Option<Uuid>, i64)>(&mut conn)
        .await
        .unwrap();

    assert_eq!(
        invitations,
        vec![
            (
                "invitation-dead".into(),
                LocalTaskStatus::Dead.as_str().into(),
                None,
                0
            ),
            (
                "invitation-processing".into(),
                LocalTaskStatus::Processing.as_str().into(),
                Some(Uuid::from_u128(1)),
                0
            ),
        ]
    );
}
