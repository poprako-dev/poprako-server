#![allow(
    clippy::unwrap_used,
    reason = "Database fixture setup and regression assertions fail immediately"
)]

//! Physical snapshots and revision fencing through generated typed dispatch.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use diesel::{ExpressionMethods as _, QueryDsl as _};
use diesel_async::RunQueryDsl as _;
use time::{Duration, OffsetDateTime};

use poprako_obj_dept::key::ObjPath;
use poprako_obj_dept::model::record::ObjKeyRecord;
use poprako_obj_dept::model::slot::ObjDeptPoolSlot;
use poprako_obj_dept::model::task::{
    CHECK, DELETE, ObjDeptPromTask, ObjTaskAction, obj_task_id, validate_task,
};
use poprako_obj_dept::model::url::{ObjUrlSpec, ObjUrls};
use poprako_obj_dept::pool::{ObjDeptPool, ObjDeptPoolView};
use poprako_obj_dept::rest::{ObjDeptError, ObjDeptRest};
use poprako_rdb_core::RdbCore;

use crate::part_impl::obj_dept::{NormObjDept, RdbObjDeptProm};
use crate::part_impl::repo::rdb_impl::schema::t_page_source_image;
use crate::shared::test_rdb::start;

const ID: &str = "page_source--00000007001";
const OLD_PATH: &str =
    "page_source/chapter--00000000001/page_source--00000007001/1.png";
const CURRENT_PATH: &str =
    "page_source/chapter--00000000002/page_source--00000007001/2.png";

struct ProbeState {
    core: RdbCore,

    exists: AtomicBool,
    mark_during_head: AtomicBool,

    calls: Mutex<Vec<(&'static str, String)>>,
}

#[derive(Clone)]
struct ProbePool(Arc<ProbeState>);

impl ObjDeptPoolView for ProbePool {
    async fn gen_urls(
        &self,
        _key: &ObjPath,
        _spec: ObjUrlSpec,
    ) -> ObjDeptRest<ObjUrls> {
        Err(ObjDeptError::Unrecoverable {
            message: "read URLs are outside this fixture".into(),
        })
    }

    async fn has(&self, key: &ObjPath) -> ObjDeptRest<bool> {
        self.0
            .calls
            .lock()
            .unwrap()
            .push(("has", key.as_str().into()));

        if self.0.mark_during_head.swap(false, Ordering::SeqCst) {
            let mut conn = self.0.core.get().await.unwrap();

            diesel::update(
                t_page_source_image::table
                    .filter(t_page_source_image::f_id.eq(ID)),
            )
            .set((
                t_page_source_image::f_is_uploaded.eq(true),
                t_page_source_image::f_updated_at
                    .eq(OffsetDateTime::now_utc() + Duration::seconds(1)),
            ))
            .execute(&mut conn)
            .await
            .unwrap();
        }

        Ok(AtomicBool::load(&self.0.exists, Ordering::SeqCst))
    }
}

impl ObjDeptPool for ProbePool {
    async fn gen_slot(
        &self,
        _key: &ObjPath,
        _content_type: &str,
        _byte_len: u64,
    ) -> ObjDeptRest<ObjDeptPoolSlot> {
        Err(ObjDeptError::Unrecoverable {
            message: "upload signing is outside this fixture".into(),
        })
    }

    async fn del(&self, key: &ObjPath) -> ObjDeptRest<()> {
        self.0
            .calls
            .lock()
            .unwrap()
            .push(("del", key.as_str().into()));

        Ok(())
    }
}

async fn dispatch(
    core: &RdbCore,
    pool: &ProbePool,
    oper: &str,
    ver: u32,
    chapter_id: &str,
) -> ObjTaskAction {
    let key = ObjKeyRecord {
        id: ID.into(),
        ver,
        domain: serde_json::json!({
            "chapter_id": chapter_id,
            "page_source_id": ID,
            "ext": "png",
        }),
    };

    let task = ObjDeptPromTask {
        id: obj_task_id("page_source_image", oper, &key, 0),
        topic: "page_source_image".into(),
        oper: oper.into(),
        obj_id: key.id,
        ver: i64::from(ver),
        domain: key.domain,
        gen_no: 0,
        retried_count: 0,
        claim_token: uuid::Uuid::from_u128(7),
    };

    validate_task(&task).unwrap();

    NormObjDept::<ProbePool, RdbObjDeptProm>::dispatch(
        core.clone(),
        pool.clone(),
        task,
    )
    .await
    .unwrap()
}

async fn is_uploaded(core: &RdbCore) -> bool {
    let mut conn = core.get().await.unwrap();

    t_page_source_image::table
        .filter(t_page_source_image::f_id.eq(ID))
        .select(t_page_source_image::f_is_uploaded)
        .first::<Option<bool>>(&mut conn)
        .await
        .unwrap()
        .unwrap()
}

// dispatch(Check, Delete)(positive,negative): canonical generations survive cleanup; current-generation conflicts and concurrent confirmation retain their guards.
#[tokio::test]
#[serial_test::serial(repo_rdb)]
async fn typed_domains_preserve_cleanup_and_revision_fencing() {
    let database = start().await;

    let core = database.core();

    let pool = ProbePool(Arc::new(ProbeState {
        core: core.clone(),
        exists: AtomicBool::new(false),
        mark_during_head: AtomicBool::new(false),
        calls: Mutex::default(),
    }));

    assert!(matches!(
        dispatch(&core, &pool, CHECK, 1, "chapter--00000000001").await,
        ObjTaskAction::Complete
    ));

    let mut conn = core.get().await.unwrap();

    diesel::insert_into(t_page_source_image::table)
        .values((
            t_page_source_image::f_id.eq(ID),
            t_page_source_image::f_version.eq(2_i64),
            t_page_source_image::f_domain.eq(serde_json::json!({
                "chapter_id": "chapter--00000000002",
                "page_source_id": ID,
                "ext": "png",
            })),
            t_page_source_image::f_is_uploaded.eq(true),
            t_page_source_image::f_hash.eq(vec![7_u8; 32]),
            t_page_source_image::f_ext.eq("png"),
        ))
        .execute(&mut conn)
        .await
        .unwrap();

    drop(conn);

    assert!(matches!(
        dispatch(&core, &pool, DELETE, 1, "chapter--00000000001").await,
        ObjTaskAction::Complete
    ));

    assert!(matches!(
        dispatch(&core, &pool, CHECK, 2, "chapter--00000000001").await,
        ObjTaskAction::Operator { .. }
    ));

    assert!(matches!(
        dispatch(&core, &pool, DELETE, 2, "chapter--00000000002").await,
        ObjTaskAction::Operator { .. }
    ));

    assert!(matches!(
        dispatch(&core, &pool, CHECK, 3, "chapter--00000000002").await,
        ObjTaskAction::Operator { .. }
    ));

    assert!(matches!(
        dispatch(&core, &pool, CHECK, 2, "chapter--00000000002").await,
        ObjTaskAction::Complete
    ));

    assert!(!is_uploaded(&core).await);

    pool.0.exists.store(true, Ordering::SeqCst);

    assert!(matches!(
        dispatch(&core, &pool, CHECK, 2, "chapter--00000000002").await,
        ObjTaskAction::Complete
    ));

    assert!(is_uploaded(&core).await);

    pool.0.exists.store(false, Ordering::SeqCst);

    pool.0.mark_during_head.store(true, Ordering::SeqCst);

    assert!(matches!(
        dispatch(&core, &pool, CHECK, 2, "chapter--00000000002").await,
        ObjTaskAction::Retry { .. }
    ));

    assert!(is_uploaded(&core).await);

    assert_eq!(
        *pool.0.calls.lock().unwrap(),
        vec![
            ("del", OLD_PATH.into()),
            ("del", OLD_PATH.into()),
            ("has", CURRENT_PATH.into()),
            ("has", CURRENT_PATH.into()),
            ("has", CURRENT_PATH.into()),
        ]
    );
}
