mod test_support;

use uuid::Uuid;

use poprako_orchestra::{Context, Level, Oper, Run, Step};

#[cfg(feature = "rdb_impl")]
use poprako_rdb_core::RdbError;

#[cfg(feature = "rdb_impl")]
use crate::actor::rdb_impl::{
    ObjKeyState, classify, presence_cas_conflict_requires_retry,
    requires_presence_reconciliation,
};
use crate::model::alloc::ObjAlloc;
use crate::model::meta::ObjMeta;
use crate::model::task::{CHECK, ObjDeptPromTask, obj_task_id, validate_task};
use crate::model::url::ObjUrls;
use crate::oper::{
    ClearObjs, DeleteObjs, GenObjSlot, GenObjSlots, GenObjUrls, ListObjMetas,
    MarkObjUploaded,
};
use crate::rest::ObjDeptError;
use crate::{ObjDept, ObjDeptView};

use self::test_support::{PageSourceImage, Text};

struct TestLevel;

impl Level for TestLevel {}

struct TestContext;

impl Context for TestContext {
    type Level = TestLevel;
}

struct TestDept;

impl<'a> Run<ListObjMetas<'a, PageSourceImage>> for TestDept {
    type Error = ObjDeptError;

    async fn run(
        &self,
        _oper: &ListObjMetas<'a, PageSourceImage>,
    ) -> Result<
        std::collections::HashMap<Text, ObjMeta<PageSourceImage>>,
        Self::Error,
    > {
        Ok(std::collections::HashMap::default())
    }
}

impl<'a> Run<GenObjUrls<'a, PageSourceImage>> for TestDept {
    type Error = ObjDeptError;

    async fn run(
        &self,
        _oper: &GenObjUrls<'a, PageSourceImage>,
    ) -> Result<std::collections::HashMap<Text, ObjUrls>, Self::Error> {
        Ok(std::collections::HashMap::default())
    }
}

impl<'a> Run<MarkObjUploaded<'a, PageSourceImage>> for TestDept {
    type Error = ObjDeptError;

    async fn run(
        &self,
        _oper: &MarkObjUploaded<'a, PageSourceImage>,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

impl<'a> Step<MarkObjUploaded<'a, PageSourceImage>, TestContext> for TestDept {
    type Level = TestLevel;
    type Error = ObjDeptError;

    async fn step(
        &self,
        _context: &mut TestContext,
        _oper: &MarkObjUploaded<'a, PageSourceImage>,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

impl<'a> Step<ListObjMetas<'a, PageSourceImage>, TestContext> for TestDept {
    type Level = TestLevel;
    type Error = ObjDeptError;

    async fn step(
        &self,
        _context: &mut TestContext,
        _oper: &ListObjMetas<'a, PageSourceImage>,
    ) -> Result<
        std::collections::HashMap<Text, ObjMeta<PageSourceImage>>,
        Self::Error,
    > {
        Ok(std::collections::HashMap::default())
    }
}

impl<'a> Step<GenObjSlot<'a, PageSourceImage>, TestContext> for TestDept {
    type Level = TestLevel;
    type Error = ObjDeptError;

    async fn step(
        &self,
        _context: &mut TestContext,
        _oper: &GenObjSlot<'a, PageSourceImage>,
    ) -> Result<ObjAlloc<PageSourceImage>, Self::Error> {
        //
        Err(ObjDeptError::Unrecoverable {
            message: "compile-only slot".into(),
        })
    }
}

impl<'a> Step<GenObjSlots<'a, PageSourceImage>, TestContext> for TestDept {
    type Level = TestLevel;
    type Error = ObjDeptError;

    async fn step(
        &self,
        _context: &mut TestContext,
        _oper: &GenObjSlots<'a, PageSourceImage>,
    ) -> Result<
        std::collections::HashMap<Text, ObjAlloc<PageSourceImage>>,
        Self::Error,
    > {
        Ok(std::collections::HashMap::default())
    }
}

impl<'a> Step<ClearObjs<'a, PageSourceImage>, TestContext> for TestDept {
    type Level = TestLevel;
    type Error = ObjDeptError;

    async fn step(
        &self,
        _context: &mut TestContext,
        _oper: &ClearObjs<'a, PageSourceImage>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<'a> Step<DeleteObjs<'a, PageSourceImage>, TestContext> for TestDept {
    type Level = TestLevel;
    type Error = ObjDeptError;

    async fn step(
        &self,
        _context: &mut TestContext,
        _oper: &DeleteObjs<'a, PageSourceImage>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

fn require_oper_output<O, T>()
where
    O: Oper<Output = T>,
{
    //
}

fn require_obj_dept<D>()
where
    D: ObjDept<PageSourceImage, TestContext>,
{
    //
}

fn require_obj_dept_view<D>()
where
    D: ObjDeptView<PageSourceImage, TestContext>,
{
    //
}

#[test]
fn operations_keep_marker_in_their_type_identity() {
    //
    require_oper_output::<
        ListObjMetas<'static, PageSourceImage>,
        std::collections::HashMap<Text, ObjMeta<PageSourceImage>>,
    >();

    require_oper_output::<
        GenObjUrls<'static, PageSourceImage>,
        std::collections::HashMap<Text, ObjUrls>,
    >();

    require_oper_output::<
        GenObjSlot<'static, PageSourceImage>,
        ObjAlloc<PageSourceImage>,
    >();

    require_oper_output::<MarkObjUploaded<'static, PageSourceImage>, bool>();

    require_oper_output::<
        GenObjSlots<'static, PageSourceImage>,
        std::collections::HashMap<Text, ObjAlloc<PageSourceImage>>,
    >();

    require_oper_output::<ClearObjs<'static, PageSourceImage>, ()>();

    require_oper_output::<DeleteObjs<'static, PageSourceImage>, ()>();
}

#[test]
fn obj_dept_aggregates_the_locked_run_and_step_capabilities() {
    require_obj_dept::<TestDept>();
}

#[test]
fn obj_dept_view_aggregates_only_read_capabilities() {
    require_obj_dept_view::<TestDept>();
}

#[test]
fn cleanup_operations_retain_their_input_ids() {
    //
    let ids = vec![
        Text::from("page_source--00000000001"),
        Text::from("page_source--00000000002"),
    ];

    let clear = ClearObjs::<PageSourceImage>::new(&ids);

    let delete = DeleteObjs::<PageSourceImage>::new(&ids);

    assert_eq!(clear.ids, ids);

    assert_eq!(delete.ids, ids);
}

fn task() -> ObjDeptPromTask {
    let key = crate::model::record::ObjKeyRecord {
        id: "page_source--00000000001".into(),
        ver: 7,
        domain: serde_json::json!({"id": "page-1", "ext": "png"}),
    };

    ObjDeptPromTask {
        id: obj_task_id("page_source_image", CHECK, &key, 2),
        topic: "page_source_image".into(),
        oper: CHECK.into(),
        obj_id: key.id,
        ver: i64::from(key.ver),
        domain: key.domain,
        gen_no: 2,
        retried_count: 1,
        claim_token: Uuid::from_u128(11),
    }
}

#[test]
fn task_envelope_accepts_consistent_fields() {
    assert!(validate_task(&task()).is_ok());
}

#[test]
fn task_envelope_rejects_invalid_identity_and_counts() {
    let mut wrong_id = task();

    wrong_id.id = "wrong".into();

    assert!(validate_task(&wrong_id).is_err());

    let mut negative_generation = task();

    negative_generation.gen_no = -1;

    assert!(validate_task(&negative_generation).is_err());

    let mut negative_retried_count = task();

    negative_retried_count.retried_count = -1;

    assert!(validate_task(&negative_retried_count).is_err());

    let mut nil_token = task();

    nil_token.claim_token = Uuid::nil();

    assert!(validate_task(&nil_token).is_err());
}

#[cfg(feature = "rdb_impl")]
#[test]
fn unavailable_and_available_generations_require_presence_reconciliation() {
    assert!(requires_presence_reconciliation(ObjKeyState::Unavailable,));

    assert!(requires_presence_reconciliation(ObjKeyState::Avail));

    assert!(!requires_presence_reconciliation(ObjKeyState::Retired));
}

#[cfg(feature = "rdb_impl")]
#[test]
fn concurrent_mark_after_absent_head_retries_presence_reconciliation() {
    assert!(presence_cas_conflict_requires_retry(ObjKeyState::Avail,));
}

#[cfg(feature = "rdb_impl")]
#[test]
fn newer_generation_after_head_does_not_retry_old_presence_update() {
    assert!(!presence_cas_conflict_requires_retry(ObjKeyState::Stale));
}

#[cfg(feature = "rdb_impl")]
#[test]
fn rdb_pool_wait_timeout_preserves_unavailable_classification() {
    let error = crate::rdb_impl::rdb_error(RdbError::PoolWaitTimeout);

    assert!(matches!(error, ObjDeptError::Unavailable { .. }));
}

#[cfg(feature = "rdb_impl")]
#[test]
fn upload_evidence_classification_preserves_active_metadata()
-> crate::rest::ObjDeptRest<()> {
    let unavailable = crate::rdb_impl::ObjRdbRecord {
        ver: 4,
        domain: Some(serde_json::json!({"id": "page-1", "ext": "png"})),
        f_is_uploaded: Some(false),
        hash: Some(vec![7; 32]),
        ext: Some(String::from("png")),
    };
    let available = crate::rdb_impl::ObjRdbRecord {
        f_is_uploaded: Some(true),
        ..unavailable.clone()
    };

    assert_eq!(classify(4, Some(&unavailable))?, ObjKeyState::Unavailable,);

    assert_eq!(classify(4, Some(&available))?, ObjKeyState::Avail);

    assert_eq!(unavailable.hash, available.hash);

    assert_eq!(unavailable.ext, available.ext);

    Ok(())
}
