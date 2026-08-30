use super::*;

use poprako_orchestra::{Nucl as _, OperRun as _, OperStep as _};

use poprako_obj_dept::model::slot::ObjSlotSpec;
use poprako_obj_dept::obj_inst;
use poprako_obj_dept::oper::MarkObjUploadedOutcome;
use poprako_obj_dept::pool::ObjUrlProfile;

use crate::part::obj_dept::PageImage;

#[test]
fn origin_only_profile_omits_thumbnail() {
    let meta = ObjMeta {
        key: ObjKey {
            id: String::from("page-1"),
            version: 1,
        },
        is_available: true,
        hash: vec![1; 32],
        ext: String::from("png"),
    };

    let urls =
        gen_urls("font_file", Some(&meta), ObjUrlProfile::OriginOnly, true)
            .unwrap()
            .unwrap();

    assert!(urls.thumbnail_url.is_none());
}

#[test]
fn image_thumbnail_profile_generates_thumbnail() {
    let meta = ObjMeta {
        key: ObjKey {
            id: String::from("page-1"),
            version: 1,
        },
        is_available: true,
        hash: vec![1; 32],
        ext: String::from("png"),
    };

    let urls = gen_urls(
        "page_image",
        Some(&meta),
        ObjUrlProfile::ImageThumbnail,
        true,
    )
    .unwrap()
    .unwrap();

    assert!(urls.thumbnail_url.is_some());
}

#[tokio::test]
async fn mock_operation_can_disable_thumbnails() {
    let mock = Mock::new().with_obj_thumbnail_disabled();
    let meta = ObjMeta {
        key: ObjKey {
            id: String::from("page-1"),
            version: 1,
        },
        is_available: true,
        hash: vec![1; 32],
        ext: String::from("png"),
    };
    let metas = HashMap::from([(String::from("page-1"), meta)]);

    let urls = obj_inst! { GenObjUrls<PageImage> { metas: &metas } }
        .run_on(&mock)
        .await
        .unwrap();

    assert!(matches!(
        urls.get("page-1"),
        Some(ObjUrls {
            thumbnail_url: None,
            ..
        }),
    ));
}

#[tokio::test]
async fn upload_mark_is_exact_current_and_idempotent() {
    let mock = Mock::new();
    let key = ObjKey {
        id: String::from("page-1"),
        version: 4,
    };
    let meta = ObjMeta {
        key: key.clone(),
        is_available: false,
        hash: vec![4; 32],
        ext: String::from("png"),
    };

    mock.state
        .lock()
        .unwrap()
        .objs
        .entry("page_image")
        .or_default()
        .insert(
            key.id.clone(),
            MockObjRecord {
                version: key.version,
                meta: Some(meta),
            },
        );

    let first = obj_inst! { MarkObjUploaded<PageImage> { key: &key } }
        .run_on(&mock)
        .await
        .unwrap();
    let second = obj_inst! { MarkObjUploaded<PageImage> { key: &key } }
        .run_on(&mock)
        .await
        .unwrap();
    let stale_key = ObjKey {
        version: 3,
        ..key.clone()
    };
    let stale = obj_inst! { MarkObjUploaded<PageImage> { key: &stale_key } }
        .run_on(&mock)
        .await
        .unwrap();
    let missing_key = ObjKey {
        id: String::from("missing"),
        version: 1,
    };
    let missing = obj_inst! {
        MarkObjUploaded<PageImage> { key: &missing_key }
    }
    .run_on(&mock)
    .await
    .unwrap();

    assert_eq!(first, MarkObjUploadedOutcome::Marked);

    assert_eq!(second, MarkObjUploadedOutcome::Marked);

    assert_eq!(stale, MarkObjUploadedOutcome::NotCurrent);

    assert_eq!(missing, MarkObjUploadedOutcome::NotCurrent);

    let snapshot = mock.snapshot();
    let uploaded = snapshot
        .objs
        .get("page_image")
        .and_then(|objs| objs.get("page-1"))
        .and_then(|record| record.meta.as_ref())
        .is_some_and(|meta| meta.is_available);

    assert!(uploaded);

    let mut state = mock.state.lock().unwrap();

    let detached_prepared = match state
        .objs
        .get_mut("page_image")
        .and_then(|objs| objs.get_mut("page-1"))
    {
        Some(record) => {
            record.meta = None;

            true
        }
        None => false,
    };

    drop(state);

    assert!(detached_prepared);

    let detached = obj_inst! { MarkObjUploaded<PageImage> { key: &key } }
        .run_on(&mock)
        .await
        .unwrap();

    assert_eq!(detached, MarkObjUploadedOutcome::NotCurrent);
}

#[tokio::test]
async fn slot_and_remove_defer_check_and_delete_debt() {
    let mock = Mock::new();

    let obj_dept = mock.clone();

    mock.coord(async move |context| {
        //
        let obj_spec = ObjSlotSpec {
            id: "page-1",
            hash: &[1; 32],
            ext: "png",
            content_type: "image/png",
            byte_len: 1024,
        };

        obj_inst! { GenObjSlot<PageImage> { spec: &obj_spec } }
            .step_on(&obj_dept, context)
            .await?;

        let ids = vec![String::from("page-1")];

        obj_inst! { RetireObjs<PageImage>::RemoveRows { ids: &ids } }
            .step_on(&obj_dept, context)
            .await?;

        Ok::<(), ObjDeptError>(())
    })
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert!(
        snapshot
            .objs
            .get("page_image")
            .is_none_or(HashMap::is_empty)
    );

    assert_eq!(snapshot.obj_tasks.len(), 2);

    assert!(matches!(snapshot.obj_tasks[0].1, ObjTask::Check { .. }));

    assert!(matches!(snapshot.obj_tasks[1].1, ObjTask::Delete { .. }));
}

#[tokio::test]
async fn batch_slots_reject_duplicate_ids_before_mutation() {
    let mock = Mock::new();
    let obj_dept = mock.clone();

    let result = mock
        .coord(async move |context| {
            let specs = [
                ObjSlotSpec {
                    id: "page-1",
                    hash: &[1; 32],
                    ext: "png",
                    content_type: "image/png",
                    byte_len: 1024,
                },
                ObjSlotSpec {
                    id: "page-1",
                    hash: &[2; 32],
                    ext: "png",
                    content_type: "image/png",
                    byte_len: 2048,
                },
            ];

            obj_inst! { GenObjSlots<PageImage> { specs: &specs } }
                .step_on(&obj_dept, context)
                .await
        })
        .await;

    assert!(matches!(
        result,
        Err(poprako_orchestra::nucl::Error::Step(
            ObjDeptError::Invalid { .. }
        )),
    ));

    let snapshot = mock.snapshot();

    assert!(snapshot.objs.get("page_image").is_none());

    assert!(snapshot.obj_tasks.is_empty());
}
