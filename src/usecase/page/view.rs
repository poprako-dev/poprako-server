//! Page presentation assembly.

use std::collections::HashMap;

use poprako_orchestra::{Context, OperRun as _};

use poprako_obj_dept::ObjDeptView;
use poprako_obj_dept::model::meta::ObjMeta;
use poprako_obj_dept::model::url::{ObjUrlSpec, ObjUrls};
use poprako_obj_dept::oper::{GenObjUrls, ListObjMetas};

use crate::data::view::page::PageInfoView;
use crate::model::read::proj::page::PageInfo;
use crate::part::obj_dept::PageImage;
use crate::result::{BaseError, BaseRest, accept};

// Image metadata and URLs resolved from one object-version snapshot.
struct PageImageData {
    //
    // Metadata by page identifier.
    obj_metas: HashMap<String, ObjMeta>,

    // Image URLs by page identifier.
    image_urls: HashMap<String, ObjUrls>,
}

/// Resolves one page model with its image metadata and selected image URLs.
pub async fn page_info_view<C, O>(
    obj_dept: &O,
    model: PageInfo,
) -> BaseRest<PageInfoView>
where
    C: Context,
    O: ObjDeptView<PageImage, C> + Sync,
{
    let page_ids = [model.id.as_str()];

    let image_data = load_image_data::<C, O>(obj_dept, &page_ids).await?;

    let obj_meta = image_data.obj_metas.get(&model.id);

    let image_urls = image_data.image_urls.get(&model.id);

    accept(PageInfoView::from_model(
        model,
        obj_meta,
        image_urls
            .and_then(|urls| urls.origin_url.as_ref())
            .map(ToString::to_string),
        image_urls
            .and_then(|urls| urls.optimized_url.as_ref())
            .map(ToString::to_string),
        image_urls
            .and_then(|urls| urls.thumbnail_url.as_ref())
            .map(ToString::to_string),
    ))
}

/// Resolves page models from one image-metadata snapshot.
pub async fn page_info_views<C, O>(
    obj_dept: &O,
    models: Vec<PageInfo>,
) -> BaseRest<Vec<PageInfoView>>
where
    C: Context,
    O: ObjDeptView<PageImage, C> + Sync,
{
    let page_ids = models
        .iter()
        .map(|model| model.id.as_str())
        .collect::<Vec<_>>();

    let image_data = load_image_data::<C, O>(obj_dept, &page_ids).await?;

    accept(
        models
            .into_iter()
            .map(|model| {
                //
                let obj_meta = image_data.obj_metas.get(&model.id);

                let image_urls = image_data.image_urls.get(&model.id);

                PageInfoView::from_model(
                    model,
                    obj_meta,
                    image_urls
                        .and_then(|urls| urls.origin_url.as_ref())
                        .map(ToString::to_string),
                    image_urls
                        .and_then(|urls| urls.optimized_url.as_ref())
                        .map(ToString::to_string),
                    image_urls
                        .and_then(|urls| urls.thumbnail_url.as_ref())
                        .map(ToString::to_string),
                )
            })
            .collect(),
    )
}

// Loads one consistent image metadata snapshot and its matching URLs.
async fn load_image_data<C, O>(
    obj_dept: &O,
    page_ids: &[&str],
) -> BaseRest<PageImageData>
where
    C: Context,
    O: ObjDeptView<PageImage, C> + Sync,
{
    if page_ids.is_empty() {
        //
        return accept(PageImageData {
            obj_metas: HashMap::new(),
            image_urls: HashMap::new(),
        });
    }

    let mut page_ids = page_ids.to_vec();

    page_ids.sort_unstable();

    page_ids.dedup();

    let obj_metas = ListObjMetas::<PageImage>::new(&page_ids)
        .run_on(obj_dept)
        .await
        .map_err(BaseError::from)?;

    let obj_url_spec = ObjUrlSpec::default()
        .with_origin()
        .with_optimized()
        .with_thumbnail();

    let image_urls = GenObjUrls::<PageImage>::new(&obj_metas, obj_url_spec)
        .run_on(obj_dept)
        .await
        .map_err(BaseError::from)?;

    accept(PageImageData {
        obj_metas,
        image_urls,
    })
}
