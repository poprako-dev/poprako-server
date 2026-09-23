//! Shared metadata and consumable URL batches for response object markers.

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::sync::Arc;

use poprako_orchestra::{Context, OperRun as _};

use poprako_obj_dept::ObjDeptView;
use poprako_obj_dept::key::KeyMap;
use poprako_obj_dept::model::url::{ObjUrlSpec, ObjUrls};
use poprako_obj_dept::oper::{GenObjUrls, ListObjMetas};

use crate::data::view::obj_url::ObjUrlView;
use crate::result::{BaseError, BaseRest, accept};

/// Origin and thumbnail presentation text for one object.
pub type ObjUrlViews = (Option<ObjUrlView>, Option<ObjUrlView>);

// Only repeated objects allocate shared handles for their URL text.
enum CachedObjUrls {
    /// URL text needed by one consumer.
    Single {
        // Owned origin and thumbnail text.
        urls: ObjUrlViews,
    },

    /// URL text shared until its final consumer.
    Repeated {
        // Number of consumers still awaiting their handles.
        remaining: usize,
        // Shared origin URL buffer.
        origin: Option<Arc<String>>,
        // Shared thumbnail URL buffer.
        thumbnail: Option<Arc<String>>,
    },
}

/// Object URLs consumed once for each occurrence in a presentation batch.
#[derive(Default)]
pub struct ObjUrlBatch {
    /// Generated object identifiers retain ownership of their cache keys.
    urls: HashMap<String, CachedObjUrls>,
}

impl ObjUrlBatch {
    /// Moves single-use text or the final shared handle to its consumer.
    pub fn take(&mut self, id: &str) -> Option<ObjUrlViews> {
        //
        if let Some(CachedObjUrls::Repeated {
            remaining,
            origin,
            thumbnail,
        }) = self.urls.get_mut(id)
            && *remaining > 1
        {
            //
            *remaining -= 1;

            return Some((
                origin.as_ref().map(Arc::clone).map(ObjUrlView::from),
                thumbnail.as_ref().map(Arc::clone).map(ObjUrlView::from),
            ));
        }

        match self.urls.remove(id)? {
            //
            CachedObjUrls::Single { urls } => Some(urls),

            CachedObjUrls::Repeated {
                origin, thumbnail, ..
            } => Some((
                origin.map(ObjUrlView::from),
                thumbnail.map(ObjUrlView::from),
            )),
        }
    }

    // Consumes generated URLs without copying their text allocations.
    fn from_urls(
        urls: HashMap<String, ObjUrls>,
        uses: &HashMap<&str, usize>,
    ) -> Self {
        //
        let urls = urls
            .into_iter()
            .filter_map(|(id, urls)| {
                //
                let remaining = uses.get(id.as_str()).copied()?;

                let origin = urls.origin_url.map(String::from);

                let thumbnail = urls.thumbnail_url.map(String::from);

                let cached = match remaining {
                    //
                    0 => return None,

                    1 => CachedObjUrls::Single {
                        urls: (
                            origin.map(ObjUrlView::from),
                            thumbnail.map(ObjUrlView::from),
                        ),
                    },

                    remaining => CachedObjUrls::Repeated {
                        remaining,
                        origin: origin.map(Arc::new),
                        thumbnail: thumbnail.map(Arc::new),
                    },
                };

                Some((id, cached))
            })
            .collect();

        Self { urls }
    }
}

/// Loads URLs once per marker while retaining every requested consumption count.
pub async fn load_obj_urls<C, O, K>(
    obj_dept: &O,
    mut ids: Vec<&str>,
) -> BaseRest<ObjUrlBatch>
where
    C: Context,
    K: KeyMap,
    O: ObjDeptView<K, C> + Sync,
{
    if ids.is_empty() {
        return accept(ObjUrlBatch::default());
    }

    let mut uses = HashMap::new();

    for id in &ids {
        *uses.entry(*id).or_insert(0) += 1;
    }

    ids.sort_unstable();

    ids.dedup();

    let obj_metas = ListObjMetas::new(&ids)
        .run_on(obj_dept)
        .await
        .map_err(BaseError::from)?;

    let obj_url_spec = ObjUrlSpec::default().with_origin().with_thumbnail();

    let obj_urls = GenObjUrls::new(&obj_metas, obj_url_spec)
        .run_on(obj_dept)
        .await
        .map_err(BaseError::from)?;

    accept(ObjUrlBatch::from_urls(obj_urls, &uses))
}
