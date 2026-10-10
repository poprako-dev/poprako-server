//! Independent composite page listing and mutation.

// Domain access loading shared by mutation entry points.
mod access;
// Shared upload allocation inside caller-owned transactions.
mod upload;

/// Composite image allocation and exact-version confirmation.
pub mod image;
/// Authoritative composite manifest replacement.
pub mod manifest;

#[cfg(test)]
mod tests;

use poprako_orchestra::{Context, OperRun as _};
use tracing::instrument;

use poprako_obj_dept::ObjDeptView;
use poprako_obj_dept::model::url::ObjUrlSpec;
use poprako_obj_dept::oper::{GenObjUrls, ListObjMetas};

use crate::data::view::page_artwork::PageArtworkInfoView;
use crate::model::shared::user::UserToken;
use crate::part::obj_dept::PageArtworkImage;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::page_artwork::ListPageArtworkInfos;
use crate::part::repo::page_artwork::PageArtworkRepo;
use crate::part::repo::team::TeamRepo;
use crate::result::{BaseError, BaseRest, accept};
use crate::usecase::page::list as page_list_usecase;

/// Reads ordered composite pages independently of source Pages.
#[instrument(level = "info", skip(repo, obj_dept, token), fields(actor_user_id = %token.user_id))]
pub async fn list_infos<C, R, O>(
    (repo, obj_dept): (&R, &O),
    token: UserToken,
    chapter_id: String,
) -> BaseRest<Vec<PageArtworkInfoView>>
where
    C: Context,
    R: PageArtworkRepo<C>
        + TeamRepo<C>
        + MemberRepo<C>
        + AssignmentRepo<C>
        + Sync,
    O: ObjDeptView<PageArtworkImage, C> + Sync,
{
    page_list_usecase::ensure_user_can_list_infos::<C, R>(
        repo,
        &token,
        &chapter_id,
    )
    .await?;

    let page_artwork_infos = ListPageArtworkInfos {
        chapter_id: &chapter_id,
    }
    .run_on(repo)
    .await?;

    let page_artwork_ids = page_artwork_infos
        .iter()
        .map(|info| info.id.as_str())
        .collect::<Vec<_>>();

    let image_metas = ListObjMetas::<PageArtworkImage>::new(&page_artwork_ids)
        .run_on(obj_dept)
        .await
        .map_err(BaseError::from)?;

    let url_spec = ObjUrlSpec::default().with_origin().with_thumbnail();

    let image_urls =
        GenObjUrls::<PageArtworkImage>::new(&image_metas, url_spec)
            .run_on(obj_dept)
            .await
            .map_err(BaseError::from)?;

    let page_artwork_info_views = page_artwork_infos
        .into_iter()
        .map(|page_artwork_info| {
            //
            let image_meta = image_metas.get(&page_artwork_info.id);

            let image_urls = image_urls.get(&page_artwork_info.id);

            PageArtworkInfoView::from_model(
                page_artwork_info,
                image_meta,
                image_urls
                    .and_then(|urls| urls.origin_url.as_ref())
                    .map(ToString::to_string),
                image_urls
                    .and_then(|urls| urls.thumbnail_url.as_ref())
                    .map(ToString::to_string),
            )
        })
        .collect();

    accept(page_artwork_info_views)
}
