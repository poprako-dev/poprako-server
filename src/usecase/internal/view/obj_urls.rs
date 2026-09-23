//! Shared metadata and URL batches for response object markers.

use std::collections::HashMap;

use poprako_orchestra::{Context, OperRun as _};

use poprako_obj_dept::ObjDeptView;
use poprako_obj_dept::key::KeyMap;
use poprako_obj_dept::model::url::{ObjUrlSpec, ObjUrls};
use poprako_obj_dept::oper::{GenObjUrls, ListObjMetas};

use crate::result::{BaseError, BaseRest, accept};

/// Loads origin and thumbnail URLs for deduplicated object marker identifiers.
pub async fn load_obj_urls<C, O, K>(
    obj_dept: &O,
    ids: &[&str],
) -> BaseRest<HashMap<String, ObjUrls>>
where
    C: Context,
    K: KeyMap,
    O: ObjDeptView<K, C> + Sync,
{
    if ids.is_empty() {
        return accept(HashMap::new());
    }

    let mut ids = ids.to_vec();

    ids.sort_unstable();

    ids.dedup();

    let obj_metas = ListObjMetas::<K>::new(&ids)
        .run_on(obj_dept)
        .await
        .map_err(BaseError::from)?;

    let obj_url_spec = ObjUrlSpec::default().with_origin().with_thumbnail();

    let obj_urls = GenObjUrls::<K>::new(&obj_metas, obj_url_spec)
        .run_on(obj_dept)
        .await
        .map_err(BaseError::from)?;

    accept(obj_urls)
}
