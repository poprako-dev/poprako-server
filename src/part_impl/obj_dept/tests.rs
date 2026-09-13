use uuid::Uuid;

use diesel::prelude::{
    ExpressionMethods as _, QueryDsl as _, TextExpressionMethods as _,
};
use diesel_async::RunQueryDsl as _;
use time::{Duration, OffsetDateTime};

use poprako_rdb_core::RdbCore;

use super::rdb_obj_dept_prom_rdb_impl::{
    claim_task, complete_task, mark_task_operator, reset_tasks, retry_task,
};
use crate::part::nucl::ReptRead;
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::obj_dept::RdbObjDeptProm;
use crate::part_impl::repo::rdb_impl::schema::t_obj_prom_task;
use poprako_obj_dept::key::ObjKey;
use poprako_obj_dept::prom::ObjDeptPromDefer as _;
use poprako_orchestra::Nucl as _;

const PREFIX: &str = "rdb-test-obj-claim-";
const PENDING: &str = "obj_prom_status:pending";
const OPERATOR: &str = "obj_prom_status:operator";

pub async fn concurrent_claim_is_unique_ordered_and_fenced(shared: RdbCore) {
    cleanup(&shared).await;

    let now = OffsetDateTime::now_utc();

    insert_task(&shared, "oldest", now - Duration::seconds(2)).await;

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
    assert!(!claimed_task.claim_token.is_nil());

    insert_task(&shared, "later", now).await;

    let later_task = claim_task(&shared).await.unwrap().unwrap();

    assert!(later_task.id.ends_with("later"));
    assert_ne!(later_task.claim_token, claimed_task.claim_token);

    cleanup(&shared).await;

    completed_tasks_can_be_recreated(&shared).await;

    repeated_defer_locks_completion(&shared).await;
}

async fn insert_task(
    shared: &RdbCore,
    suffix: &str,
    created_at: OffsetDateTime,
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
            t_obj_prom_task::f_claim_token.eq(None::<Uuid>),
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

// completed_tasks_can_be_recreated(ObjDeptProm)(negative): stale execution credentials cannot mutate a recreated task or a reclaimed attempt.
async fn completed_tasks_can_be_recreated(shared: &RdbCore) {
    let prom = RdbObjDeptProm::new(shared.clone());

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let key = ObjKey {
        id: format!("{}object", PREFIX),
        ver: 1,
        image: "test/object/1.png".into(),
    };

    let expires_at = OffsetDateTime::now_utc() - Duration::minutes(2);

    nucl.coord(async |context| {
        prom.defer_check(context, "page_image", &key, expires_at)
            .await
    })
    .await
    .unwrap();

    let old = claim_task(shared).await.unwrap().unwrap();

    // Repeated defer must retain the currently executing credential.
    nucl.coord(async |context| {
        prom.defer_check(context, "page_image", &key, expires_at)
            .await
    })
    .await
    .unwrap();

    assert_eq!(complete_task(shared, &old).await.unwrap(), 1);

    assert_eq!(complete_task(shared, &old).await.unwrap(), 0);

    nucl.coord(async |context| {
        prom.defer_check(context, "page_image", &key, expires_at)
            .await
    })
    .await
    .unwrap();

    let current = claim_task(shared).await.unwrap().unwrap();

    assert_eq!(old.id, current.id);

    assert_ne!(old.claim_token, current.claim_token);

    assert_eq!(complete_task(shared, &old).await.unwrap(), 0);

    assert_eq!(retry_task(shared, &old, "stale").await.unwrap(), 0);

    assert_eq!(mark_task_operator(shared, &old, "stale").await.unwrap(), 0);

    let mut conn = shared.get().await.unwrap();

    let stored = t_obj_prom_task::table
        .filter(t_obj_prom_task::f_id.eq(&current.id))
        .select((
            t_obj_prom_task::f_status,
            t_obj_prom_task::f_claim_token,
            t_obj_prom_task::f_error,
        ))
        .first::<(String, Option<Uuid>, Option<String>)>(&mut conn)
        .await
        .unwrap();

    assert_eq!(
        stored,
        (
            "obj_prom_status:processing".into(),
            Some(current.claim_token),
            None
        )
    );

    diesel::update(
        t_obj_prom_task::table.filter(t_obj_prom_task::f_id.eq(&current.id)),
    )
    .set(
        t_obj_prom_task::f_updated_at
            .eq(OffsetDateTime::now_utc() - Duration::minutes(4)),
    )
    .execute(&mut conn)
    .await
    .unwrap();

    assert_eq!(reset_tasks(shared).await.unwrap(), 1);

    assert_eq!(complete_task(shared, &current).await.unwrap(), 0);

    let reclaimed = claim_task(shared).await.unwrap().unwrap();

    assert_ne!(reclaimed.claim_token, current.claim_token);

    assert_eq!(retry_task(shared, &current, "stale").await.unwrap(), 0);

    assert_eq!(
        mark_task_operator(shared, &current, "stale").await.unwrap(),
        0
    );

    assert_eq!(complete_task(shared, &reclaimed).await.unwrap(), 1);

    assert_eq!(
        t_obj_prom_task::table
            .filter(t_obj_prom_task::f_obj_id.eq(&key.id))
            .count()
            .get_result::<i64>(&mut conn)
            .await
            .unwrap(),
        0
    );

    nucl.coord(async |context| {
        prom.defer_check(context, "page_image", &key, expires_at)
            .await
    })
    .await
    .unwrap();

    let failed = claim_task(shared).await.unwrap().unwrap();

    assert_eq!(
        mark_task_operator(shared, &failed, "repair").await.unwrap(),
        1
    );

    assert!(
        nucl.coord(async |context| prom
            .defer_check(context, "page_image", &key, expires_at)
            .await)
            .await
            .is_err()
    );

    let stored = t_obj_prom_task::table
        .filter(t_obj_prom_task::f_id.eq(&failed.id))
        .select((t_obj_prom_task::f_status, t_obj_prom_task::f_claim_token))
        .first::<(String, Option<Uuid>)>(&mut conn)
        .await
        .unwrap();

    assert_eq!(stored, (OPERATOR.into(), None));

    diesel::delete(
        t_obj_prom_task::table.filter(t_obj_prom_task::f_id.eq(&failed.id)),
    )
    .execute(&mut conn)
    .await
    .unwrap();
}

// repeated_defer_locks_completion(ObjDeptPromDefer)(positive): conflicting defer retains the task until identity validation and the caller transaction finish.
async fn repeated_defer_locks_completion(shared: &RdbCore) {
    let prom = RdbObjDeptProm::new(shared.clone());

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let key = ObjKey {
        id: format!("{}locked", PREFIX),
        ver: 1,
        image: "test/locked/1.png".into(),
    };

    let expires_at = OffsetDateTime::now_utc() - Duration::minutes(2);

    nucl.coord(async |context| {
        prom.defer_check(context, "page_image", &key, expires_at)
            .await
    })
    .await
    .unwrap();

    let task = claim_task(shared).await.unwrap().unwrap();

    let completion = nucl
        .coord(async |context| {
            prom.defer_check(context, "page_image", &key, expires_at)
                .await?;

            let core = shared.clone();

            let mut completion =
                tokio::spawn(async move { complete_task(&core, &task).await });

            assert!(
                tokio::time::timeout(
                    std::time::Duration::from_millis(50),
                    &mut completion
                )
                .await
                .is_err()
            );

            Ok::<_, poprako_obj_dept::rest::ObjDeptError>(completion)
        })
        .await
        .unwrap();

    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(5), completion)
            .await
            .unwrap()
            .unwrap()
            .unwrap(),
        1
    );
}

// A deterministic pool keeps RDB lifecycle tests independent of remote storage.
#[derive(Clone)]
pub struct ArtworkTestPool;

impl poprako_obj_dept::pool::ObjDeptPoolView for ArtworkTestPool {
    async fn gen_urls(
        &self,
        _key: &str,
        _spec: poprako_obj_dept::model::url::ObjUrlSpec,
    ) -> poprako_obj_dept::rest::ObjDeptRest<
        poprako_obj_dept::model::url::ObjUrls,
    > {
        Ok(poprako_obj_dept::model::url::ObjUrls {
            origin_url: None,
            optimized_url: None,
            thumbnail_url: None,
        })
    }

    async fn has(
        &self,
        _key: &str,
    ) -> poprako_obj_dept::rest::ObjDeptRest<bool> {
        Ok(true)
    }
}

impl poprako_obj_dept::pool::ObjDeptPool for ArtworkTestPool {
    async fn gen_slot(
        &self,
        key: &str,
        _content_type: &str,
        _byte_len: u64,
    ) -> poprako_obj_dept::rest::ObjDeptRest<
        poprako_obj_dept::model::slot::ObjDeptPoolSlot,
    > {
        Ok(poprako_obj_dept::model::slot::ObjDeptPoolSlot {
            url: url::Url::parse(&format!("https://obj.test/{key}")).unwrap(),
            headers: Default::default(),
            expires_at: OffsetDateTime::now_utc() + Duration::minutes(10),
        })
    }

    async fn del(&self, _key: &str) -> poprako_obj_dept::rest::ObjDeptRest<()> {
        Ok(())
    }
}

// artwork_transactional_mark(MarkObjUploaded)(negative): rollback preserves unavailable state and concurrent replacement cannot inherit an old confirmation.
pub async fn artwork_transactional_mark(shared: RdbCore) {
    use crate::part::nucl::ReptRead;
    use crate::part::obj_dept::ChapterArtwork;
    use crate::part_impl::nucl::rdb_impl::RdbNucl;
    use crate::result::{BaseError, accept};
    use crate::value::artwork::ChapterArtworkKey;
    use poprako_obj_dept::key::ObjGen;
    use poprako_obj_dept::model::slot::ObjSlotSpec;
    use poprako_obj_dept::oper::{
        ClearObjs, GenObjSlot, ListObjMetas, MarkObjUploaded,
    };
    use poprako_orchestra::{Nucl as _, OperRun as _, OperStep as _};

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let dept = super::NormObjDept::new(
        shared.clone(),
        ArtworkTestPool,
        super::RdbObjDeptProm::new(shared.clone()),
    );

    let chapter_id = "rdb-test-artwork-transaction";

    let artwork_spec = ObjSlotSpec {
        dom: ChapterArtworkKey {
            chapter_id: chapter_id.into(),
            ext: "zip".into(),
        },
        hash: &[1; 32],
        content_type: "application/octet-stream",
        byte_len: 1024,
    };

    let slot = nucl
        .coord(async |context| {
            GenObjSlot::<ChapterArtwork>::new(&artwork_spec)
                .step_on(&dept, context)
                .await
        })
        .await
        .unwrap()
        .unwrap();

    let generation = ObjGen {
        id: chapter_id.into(),
        ver: slot.key.ver,
    };

    let rollback = nucl
        .coord(async |context| {
            let marked = MarkObjUploaded::<ChapterArtwork>::new(&generation)
                .step_on(&dept, context)
                .await
                .map_err(BaseError::from)?;

            assert!(marked);

            Err::<(), _>(BaseError::Unrecoverable {
                message: "deliberate transaction failure".into(),
            })
        })
        .await;

    assert!(rollback.is_err());

    let metas = ListObjMetas::<ChapterArtwork>::new(&[chapter_id])
        .run_on(&dept)
        .await
        .unwrap();

    assert!(!metas[chapter_id].is_avail);

    let replacement_spec = ObjSlotSpec {
        hash: &[2; 32],
        ..artwork_spec
    };

    let (replacement, confirmation) = tokio::join!(
        nucl.coord(async |context| GenObjSlot::<ChapterArtwork>::new(
            &replacement_spec
        )
        .step_on(&dept, context)
        .await),
        nucl.coord(async |context| MarkObjUploaded::<ChapterArtwork>::new(
            &generation
        )
        .step_on(&dept, context)
        .await),
    );

    // Repeatable-read can abort one contender; retry only that failed operation.
    let replacement = match replacement {
        Ok(slot) => slot.unwrap(),
        Err(_) => nucl
            .coord(async |context| {
                GenObjSlot::<ChapterArtwork>::new(&replacement_spec)
                    .step_on(&dept, context)
                    .await
            })
            .await
            .unwrap()
            .unwrap(),
    };

    let _ = confirmation;

    assert!(replacement.key.ver > generation.ver);

    let metas = ListObjMetas::<ChapterArtwork>::new(&[chapter_id])
        .run_on(&dept)
        .await
        .unwrap();

    assert!(!metas[chapter_id].is_avail);

    let stale = nucl
        .coord(async |context| {
            MarkObjUploaded::<ChapterArtwork>::new(&generation)
                .step_on(&dept, context)
                .await
        })
        .await
        .unwrap();

    assert!(!stale);

    let current_generation = ObjGen {
        id: chapter_id.into(),
        ver: replacement.key.ver,
    };

    nucl.coord(async |context| {
        let marked =
            MarkObjUploaded::<ChapterArtwork>::new(&current_generation)
                .step_on(&dept, context)
                .await
                .map_err(BaseError::from)?;

        assert!(marked);

        ClearObjs::<ChapterArtwork>::new(&[chapter_id.to_owned()])
            .step_on(&dept, context)
            .await
            .map_err(BaseError::from)?;

        let marked_after_clear =
            MarkObjUploaded::<ChapterArtwork>::new(&current_generation)
                .step_on(&dept, context)
                .await
                .map_err(BaseError::from)?;

        assert!(!marked_after_clear);

        accept(())
    })
    .await
    .unwrap();
    let mut conn = shared.get().await.unwrap();

    diesel::delete(
        t_obj_prom_task::table.filter(t_obj_prom_task::f_obj_id.eq(chapter_id)),
    )
    .execute(&mut conn)
    .await
    .unwrap();

    use crate::part_impl::repo::rdb_impl::schema::t_chapter_artwork;

    diesel::delete(
        t_chapter_artwork::table.filter(t_chapter_artwork::f_id.eq(chapter_id)),
    )
    .execute(&mut conn)
    .await
    .unwrap();
}
