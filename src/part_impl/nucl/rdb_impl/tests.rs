//! Isolation, rollback, session restoration, and cancellation regressions.

use std::future::pending;
use std::time::Duration;

use diesel::define_sql_function;
use diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::Nucl as _;
use poprako_orchestra::nucl::Error as NuclError;
use time::OffsetDateTime;
use tokio::sync::oneshot;
use tokio::time::timeout;

use crate::part::nucl::{ReptRead, Serial};
use crate::part_impl::nucl::rdb_impl::{
    RdbNucl, TransactionSession, current_setting, set_config,
};
use crate::part_impl::repo::rdb_impl::schema::t_local_message;
use crate::result::BaseError;
use crate::shared::RdbContext;
use crate::shared::test_rdb;

define_sql_function! {
    /// Identifies the database session for pool reuse checks.
    fn pg_backend_pid() -> diesel::sql_types::Integer;
}

// rdb_nucl_preserves_isolation_and_session_state(coord)(positive/negative): commits and rollbacks retain isolation and restore the pooled session default.
#[tokio::test]
async fn rdb_nucl_preserves_isolation_and_session_state() {
    let database = test_rdb::start().await;

    let core = database.core();

    let mut conn = core.get().await.unwrap();

    let original = diesel::select((
        pg_backend_pid(),
        current_setting("default_transaction_isolation"),
    ))
    .get_result::<(i32, String)>(&mut conn)
    .await
    .unwrap();

    drop(conn);

    let nucl = RdbNucl::<ReptRead>::new(core.clone());

    nucl.coord(async |context| {
        let isolation =
            diesel::select(current_setting("transaction_isolation"))
                .get_result::<String>(context.conn())
                .await
                .unwrap();

        assert_eq!(isolation, "repeatable read");

        diesel::insert_into(t_local_message::table)
            .values((
                t_local_message::f_id.eq("typed-transaction"),
                t_local_message::f_topic.eq("original"),
                t_local_message::f_payload.eq(serde_json::json!({})),
                t_local_message::f_status.eq("local_message_status:pending"),
                t_local_message::f_visible_at.eq(OffsetDateTime::now_utc()),
            ))
            .execute(context.conn())
            .await
            .unwrap();

        Ok::<(), BaseError>(())
    })
    .await
    .unwrap();

    let serial = RdbNucl::<Serial>::new(core.clone());

    let result = serial
        .coord(async |context| {
            let isolation =
                diesel::select(current_setting("transaction_isolation"))
                    .get_result::<String>(context.conn())
                    .await
                    .unwrap();

            assert_eq!(isolation, "serializable");

            diesel::update(t_local_message::table)
                .set(t_local_message::f_topic.eq("rolled-back"))
                .execute(context.conn())
                .await
                .unwrap();

            Err::<(), _>("step failure")
        })
        .await;

    assert!(matches!(result, Err(NuclError::Step("step failure"))));

    let mut conn = core.get().await.unwrap();

    let restored = diesel::select((
        pg_backend_pid(),
        current_setting("default_transaction_isolation"),
    ))
    .get_result::<(i32, String)>(&mut conn)
    .await
    .unwrap();

    assert_eq!(restored, original);

    let topic = t_local_message::table
        .select(t_local_message::f_topic)
        .first::<String>(&mut conn)
        .await
        .unwrap();

    assert_eq!(topic, "original");

    drop(conn);

    let (started_send, started_recv) = oneshot::channel();

    let attempt = serial.coord(async |context| {
        diesel::update(t_local_message::table)
            .set(t_local_message::f_topic.eq("cancelled"))
            .execute(context.conn())
            .await
            .unwrap();

        started_send.send(()).unwrap();

        pending::<Result<(), BaseError>>().await
    });

    timeout(Duration::from_secs(5), async {
        tokio::select! {
            result = attempt => panic!("pending transaction finished: {:?}", result),

            started = started_recv => started.unwrap(),
        }
    })
    .await
    .unwrap();

    let mut conn = core.get().await.unwrap();

    let replacement = diesel::select((
        pg_backend_pid(),
        current_setting("default_transaction_isolation"),
    ))
    .get_result::<(i32, String)>(&mut conn)
    .await
    .unwrap();

    assert_ne!(replacement.0, original.0);

    assert_eq!(replacement.1, original.1);

    assert_eq!(
        t_local_message::table
            .select(t_local_message::f_topic)
            .first::<String>(&mut conn)
            .await
            .unwrap(),
        "original"
    );

    drop(conn);

    // Cancellation during setup must also discard the altered session, before BEGIN.
    let mut session = TransactionSession {
        context: RdbContext::<ReptRead>::new(core.get().await.unwrap()),
        restored: false,
    };

    diesel::select(set_config(
        "default_transaction_isolation",
        "serializable",
        false,
    ))
    .get_result::<String>(session.context.conn())
    .await
    .unwrap();

    drop(session);

    let mut conn = core.get().await.unwrap();

    let fresh = diesel::select((
        pg_backend_pid(),
        current_setting("default_transaction_isolation"),
    ))
    .get_result::<(i32, String)>(&mut conn)
    .await
    .unwrap();

    assert_ne!(fresh.0, replacement.0);

    assert_eq!(fresh.1, original.1);
}
