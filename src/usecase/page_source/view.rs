//! source page presentation assembly.

use std::collections::HashMap;

use poprako_orchestra::{Context, OperRun as _};

use poprako_obj_dept::ObjDeptView;
use poprako_obj_dept::key::ObjKind as _;
use poprako_obj_dept::model::meta::ObjMeta;
use poprako_obj_dept::model::url::{ObjUrlSpec, ObjUrls};
use poprako_obj_dept::oper::{GenObjUrls, ListObjMetas};

use crate::data::view::page_source::PageSourceInfoView;
use crate::model::read::proj::page_source::PageSourceInfo;
use crate::part::obj_dept::PageSourceImage;
use crate::part::repo::oper::page_source::ListPageSourceRawIdentInfos;
use crate::part::repo::page_source::PageSourceRepo;
use crate::result::{BaseError, BaseRest};
use crate::value::business_id::PageSourceId;

// Image metadata and URLs resolved from one object-version snapshot.
struct PageSourceImageData {
    // Metadata by page identifier.
    obj_metas: HashMap<PageSourceId, ObjMeta<PageSourceImage>>,

    // Image URLs by page identifier.
    image_urls: HashMap<PageSourceId, ObjUrls>,
}

/// Resolves one page model with its image metadata and selected image URLs.
pub async fn page_source_info_view<C, R, O>(
    (repo, obj_dept): (&R, &O),
    model: PageSourceInfo,
) -> BaseRest<PageSourceInfoView>
where
    C: Context,
    R: PageSourceRepo<C> + Sync,
    O: ObjDeptView<PageSourceImage, C> + Sync,
{
    let page_source_ids = [model.id.as_str()];

    let raw_idents = ListPageSourceRawIdentInfos {
        page_source_ids: &page_source_ids,
    }
    .run_on(repo)
    .await?;

    let image_data =
        load_image_data::<C, O>(obj_dept, &page_source_ids).await?;

    let obj_meta = image_data.obj_metas.get(model.id.as_str());

    let image_urls = image_data.image_urls.get(model.id.as_str());

    accept!(PageSourceInfoView::from_model(
        model,
        obj_meta,
        raw_idents.into_iter().next().map(|info| info.raw_ident),
        image_urls,
    ))
}

/// Resolves page models from one image-metadata snapshot.
pub async fn page_source_info_views<C, R, O>(
    (repo, obj_dept): (&R, &O),
    models: Vec<PageSourceInfo>,
) -> BaseRest<Vec<PageSourceInfoView>>
where
    C: Context,
    R: PageSourceRepo<C> + Sync,
    O: ObjDeptView<PageSourceImage, C> + Sync,
{
    let page_source_ids = models
        .iter()
        .map(|model| model.id.as_str())
        .collect::<Vec<_>>();

    let raw_idents = ListPageSourceRawIdentInfos {
        page_source_ids: &page_source_ids,
    }
    .run_on(repo)
    .await?;

    let image_data =
        load_image_data::<C, O>(obj_dept, &page_source_ids).await?;

    accept!(
        models
            .into_iter()
            .map(|model| {
                //
                let obj_meta = image_data.obj_metas.get(model.id.as_str());

                let image_urls = image_data.image_urls.get(model.id.as_str());

                let raw_ident = raw_idents
                    .iter()
                    .find(|info| info.page_source_id == model.id)
                    .map(|info| info.raw_ident.clone());

                PageSourceInfoView::from_model(
                    model, obj_meta, raw_ident, image_urls,
                )
            })
            .collect(),
    )
}

// Loads one consistent image metadata snapshot and its matching URLs.
async fn load_image_data<C, O>(
    obj_dept: &O,
    page_source_ids: &[&str],
) -> BaseRest<PageSourceImageData>
where
    C: Context,
    O: ObjDeptView<PageSourceImage, C> + Sync,
{
    if page_source_ids.is_empty() {
        //
        return accept!(PageSourceImageData {
            obj_metas: HashMap::new(),
            image_urls: HashMap::new(),
        });
    }

    let mut page_source_ids = page_source_ids.to_vec();

    page_source_ids.sort_unstable();

    page_source_ids.dedup();

    let page_source_image_ids = PageSourceImage::ids(&page_source_ids)?;

    let obj_metas =
        ListObjMetas::<PageSourceImage>::new(&page_source_image_ids)
            .run_on(obj_dept)
            .await
            .map_err(BaseError::from)?;

    let obj_url_spec = ObjUrlSpec::default()
        .with_origin()
        .with_optimized()
        .with_thumbnail();

    let image_urls =
        GenObjUrls::<PageSourceImage>::new(&obj_metas, obj_url_spec)
            .run_on(obj_dept)
            .await
            .map_err(BaseError::from)?;

    accept!(PageSourceImageData {
        obj_metas,
        image_urls,
    })
}
