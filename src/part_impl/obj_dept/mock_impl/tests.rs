use super::*;

use poprako_orchestra::{Nucl as _, OperStep as _};

use poprako_obj_dept::model::slot::ObjSlotSpec;
use poprako_obj_dept::obj_inst;

use crate::part::obj_dept::PageImage;

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

        obj_inst! { DelObjs<PageImage>::Remove { ids: &ids } }
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
