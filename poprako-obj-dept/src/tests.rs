use poprako_orchestra::{Context, Level, Oper, Run, Step};

use crate::ObjDept;
use crate::model::meta::ObjMeta;
use crate::model::slot::ObjSlot;
use crate::model::task::{CHECK, ObjPromTask, obj_task_id, validate_task};
use crate::obj_inst;
use crate::oper::{DelObjs, GenObjSlot, GenObjUrl, GetObjMeta};
use crate::rest::ObjDeptError;

struct PageImage;

struct TestLevel;

impl Level for TestLevel {}

struct TestContext;

impl Context for TestContext {
    type Level = TestLevel;
}

struct TestDept;

impl<'a> Run<GetObjMeta<'a, PageImage>> for TestDept {
    type Error = ObjDeptError;

    async fn run(
        &self,
        _oper: &GetObjMeta<'a, PageImage>,
    ) -> Result<Option<ObjMeta>, Self::Error> {
        Ok(None)
    }
}

impl<'a> Step<GetObjMeta<'a, PageImage>, TestContext> for TestDept {
    type Level = TestLevel;
    type Error = ObjDeptError;

    async fn step(
        &self,
        _context: &mut TestContext,
        _oper: &GetObjMeta<'a, PageImage>,
    ) -> Result<Option<ObjMeta>, Self::Error> {
        Ok(None)
    }
}

impl<'a> Run<GenObjUrl<'a, PageImage>> for TestDept {
    type Error = ObjDeptError;

    async fn run(
        &self,
        _oper: &GenObjUrl<'a, PageImage>,
    ) -> Result<Option<url::Url>, Self::Error> {
        Ok(None)
    }
}

impl<'a> Step<GenObjUrl<'a, PageImage>, TestContext> for TestDept {
    type Level = TestLevel;
    type Error = ObjDeptError;

    async fn step(
        &self,
        _context: &mut TestContext,
        _oper: &GenObjUrl<'a, PageImage>,
    ) -> Result<Option<url::Url>, Self::Error> {
        Ok(None)
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

impl<'a> Step<DelObjs<'a, PageImage>, TestContext> for TestDept {
    type Level = TestLevel;
    type Error = ObjDeptError;

    async fn step(
        &self,
        _context: &mut TestContext,
        _oper: &DelObjs<'a, PageImage>,
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

#[test]
fn operations_keep_marker_in_their_type_identity() {
    //
    require_oper_output::<GetObjMeta<'static, PageImage>, Option<ObjMeta>>();

    require_oper_output::<GenObjUrl<'static, PageImage>, Option<url::Url>>();

    require_oper_output::<GenObjSlot<'static, PageImage>, ObjSlot>();

    require_oper_output::<DelObjs<'static, PageImage>, ()>();
}

#[test]
fn obj_dept_aggregates_the_locked_run_and_step_capabilities() {
    require_obj_dept::<TestDept>();
}

#[test]
fn delete_variants_retain_their_input_ids() {
    //
    let ids = vec!["page-1".to_owned(), "page-2".to_owned()];

    let detach = obj_inst! { DelObjs<PageImage>::Detach { ids: &ids } };

    let remove = obj_inst! { DelObjs<PageImage>::Remove { ids: &ids } };

    let detach_ids = match detach {
        DelObjs::Detach { ids, .. } => ids,
        DelObjs::Remove { .. } => &[],
    };

    let remove_ids = match remove {
        DelObjs::Remove { ids, .. } => ids,
        DelObjs::Detach { .. } => &[],
    };

    assert_eq!(detach_ids, ids);

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
