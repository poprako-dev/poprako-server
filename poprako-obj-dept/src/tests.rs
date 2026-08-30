use poprako_orchestra::{Context, Level, Oper, Run, Step};

#[cfg(feature = "rdb_impl")]
use crate::actor::rdb_impl::{
    ObjKeyState, classify, presence_cas_conflict_requires_retry,
    requires_presence_reconciliation,
};
use crate::model::mark::MarkObjUploadedOutcome;
use crate::model::meta::ObjMeta;
use crate::model::slot::ObjSlot;
use crate::model::task::{CHECK, ObjPromTask, obj_task_id, validate_task};
use crate::model::url::ObjUrls;
use crate::obj_inst;
use crate::oper::{
    GenObjSlot, GenObjSlots, GenObjUrls, ListObjMetas, MarkObjUploaded,
    RetireObjs,
};
use crate::rest::ObjDeptError;
use crate::{ObjDept, ObjDeptView};

struct PageImage;

struct TestLevel;

impl Level for TestLevel {}

struct TestContext;

impl Context for TestContext {
    type Level = TestLevel;
}

struct TestDept;

impl<'a> Run<ListObjMetas<'a, PageImage>> for TestDept {
    type Error = ObjDeptError;

    async fn run(
        &self,
        _oper: &ListObjMetas<'a, PageImage>,
    ) -> Result<std::collections::HashMap<String, ObjMeta>, Self::Error> {
        Ok(std::collections::HashMap::default())
    }
}

impl<'a> Run<GenObjUrls<'a, PageImage>> for TestDept {
    type Error = ObjDeptError;

    async fn run(
        &self,
        _oper: &GenObjUrls<'a, PageImage>,
    ) -> Result<std::collections::HashMap<String, ObjUrls>, Self::Error> {
        Ok(std::collections::HashMap::default())
    }
}

impl<'a> Run<MarkObjUploaded<'a, PageImage>> for TestDept {
    type Error = ObjDeptError;

    async fn run(
        &self,
        _oper: &MarkObjUploaded<'a, PageImage>,
    ) -> Result<MarkObjUploadedOutcome, Self::Error> {
        Ok(MarkObjUploadedOutcome::Marked)
    }
}

impl<'a> Step<ListObjMetas<'a, PageImage>, TestContext> for TestDept {
    type Level = TestLevel;
    type Error = ObjDeptError;

    async fn step(
        &self,
        _context: &mut TestContext,
        _oper: &ListObjMetas<'a, PageImage>,
    ) -> Result<std::collections::HashMap<String, ObjMeta>, Self::Error> {
        Ok(std::collections::HashMap::default())
    }
}

impl<'a> Step<GenObjSlot<'a, PageImage>, TestContext> for TestDept {
    type Level = TestLevel;
    type Error = ObjDeptError;

    async fn step(
        &self,
        _context: &mut TestContext,
        _oper: &GenObjSlot<'a, PageImage>,
    ) -> Result<ObjSlot, Self::Error> {
        //
        Err(ObjDeptError::Unrecoverable {
            message: "compile-only slot".into(),
        })
    }
}

impl<'a> Step<GenObjSlots<'a, PageImage>, TestContext> for TestDept {
    type Level = TestLevel;
    type Error = ObjDeptError;

    async fn step(
        &self,
        _context: &mut TestContext,
        _oper: &GenObjSlots<'a, PageImage>,
    ) -> Result<std::collections::HashMap<String, ObjSlot>, Self::Error> {
        Ok(std::collections::HashMap::default())
    }
}

impl<'a> Step<RetireObjs<'a, PageImage>, TestContext> for TestDept {
    type Level = TestLevel;
    type Error = ObjDeptError;

    async fn step(
        &self,
        _context: &mut TestContext,
        _oper: &RetireObjs<'a, PageImage>,
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
    D: ObjDept<PageImage, TestContext>,
{
    //
}

fn require_obj_dept_view<D>()
where
    D: ObjDeptView<PageImage, TestContext>,
{
    //
}

#[test]
fn operations_keep_marker_in_their_type_identity() {
    //
    require_oper_output::<
        ListObjMetas<'static, PageImage>,
        std::collections::HashMap<String, ObjMeta>,
    >();

    require_oper_output::<
        GenObjUrls<'static, PageImage>,
        std::collections::HashMap<String, ObjUrls>,
    >();

    require_oper_output::<GenObjSlot<'static, PageImage>, ObjSlot>();

    require_oper_output::<
        MarkObjUploaded<'static, PageImage>,
        MarkObjUploadedOutcome,
    >();

    require_oper_output::<
        GenObjSlots<'static, PageImage>,
        std::collections::HashMap<String, ObjSlot>,
    >();

    require_oper_output::<RetireObjs<'static, PageImage>, ()>();
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
fn retirement_variants_retain_their_input_ids() {
    //
    let ids = vec!["page-1".to_owned(), "page-2".to_owned()];

    let preserve = obj_inst! {
        RetireObjs<PageImage>::PreserveWatermarks { ids: &ids }
    };

    let remove = obj_inst! { RetireObjs<PageImage>::RemoveRows { ids: &ids } };

    let preserve_ids = match preserve {
        RetireObjs::PreserveWatermarks { ids, .. } => ids,
        RetireObjs::RemoveRows { .. } => &[],
    };

    let remove_ids = match remove {
        RetireObjs::RemoveRows { ids, .. } => ids,
        RetireObjs::PreserveWatermarks { .. } => &[],
    };

    assert_eq!(preserve_ids, ids);

    assert_eq!(remove_ids, ids);
}

fn task() -> ObjPromTask {
    let key = crate::key::ObjKey {
        id: "page-1".into(),
        version: 7,
    };

    ObjPromTask {
        id: obj_task_id("page_image", CHECK, &key, 2),
        topic: "page_image".into(),
        oper: CHECK.into(),
        obj_id: key.id,
        version: i64::from(key.version),
        generation: 2,
        retried_count: 1,
        lease: 11,
    }
}

#[test]
fn task_envelope_accepts_consistent_fields() {
    assert!(validate_task(&task()).is_ok());
}

#[test]
fn task_envelope_rejects_invalid_identity_and_counters() {
    let mut wrong_id = task();

    wrong_id.id = "wrong".into();

    assert!(validate_task(&wrong_id).is_err());

    let mut negative_generation = task();

    negative_generation.generation = -1;

    assert!(validate_task(&negative_generation).is_err());

    let mut negative_retried_count = task();

    negative_retried_count.retried_count = -1;

    assert!(validate_task(&negative_retried_count).is_err());

    let mut zero_lease = task();

    zero_lease.lease = 0;

    assert!(validate_task(&zero_lease).is_err());
}

#[cfg(feature = "rdb_impl")]
#[test]
fn unavailable_and_available_generations_require_presence_reconciliation() {
    assert!(requires_presence_reconciliation(ObjKeyState::Unavailable,));

    assert!(requires_presence_reconciliation(ObjKeyState::Available));

    assert!(!requires_presence_reconciliation(ObjKeyState::Retired));
}

#[cfg(feature = "rdb_impl")]
#[test]
fn concurrent_mark_after_absent_head_retries_presence_reconciliation() {
    assert!(presence_cas_conflict_requires_retry(ObjKeyState::Available,));
}

#[cfg(feature = "rdb_impl")]
#[test]
fn newer_generation_after_head_does_not_retry_old_presence_update() {
    assert!(!presence_cas_conflict_requires_retry(ObjKeyState::Stale));
}

#[cfg(feature = "rdb_impl")]
#[test]
fn upload_evidence_classification_preserves_active_metadata()
-> crate::rest::ObjDeptRest<()> {
    let unavailable = crate::rdb_impl::ObjRdbRow {
        version: 4,
        f_is_uploaded: Some(false),
        hash: Some(vec![7; 32]),
        ext: Some(String::from("png")),
    };
    let available = crate::rdb_impl::ObjRdbRow {
        f_is_uploaded: Some(true),
        ..unavailable.clone()
    };

    assert_eq!(classify(4, Some(&unavailable))?, ObjKeyState::Unavailable,);

    assert_eq!(classify(4, Some(&available))?, ObjKeyState::Available);

    assert_eq!(unavailable.hash, available.hash);

    assert_eq!(unavailable.ext, available.ext);

    Ok(())
}
