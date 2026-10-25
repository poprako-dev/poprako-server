//! Relational hierarchy sweep orchestration.

use poprako_orchestra::{AtLeast, Context, Nucl, OperStep as _};
use tracing::instrument;

use poprako_obj_dept::ObjDept;
use poprako_obj_dept::key::ObjKind as _;
use poprako_obj_dept::oper::DeleteObjs;

use crate::model::read::proj::subtree_delete::SubtreeDeleteSweepTarget;
use crate::part::nucl::ReptRead;
use crate::part::obj_dept::{
    ChapterArtwork, ComicCover, PageArtworkImage, PageSourceImage, TeamAvatar,
};
use crate::part::repo::oper::page_artwork::ListPageArtworkInfos;
use crate::part::repo::oper::subtree_delete::{
    ClaimSubtreeSweep, ListSubtreePageSourceIds, SweepSubtree,
};
use crate::part::repo::page_artwork::PageArtworkRepo;
use crate::part::repo::subtree_delete::SubtreeRepo;
use crate::result::{BaseError, BaseRest};
use crate::value::subtree_delete::SubtreeSweepLevel;

/// Sweeps one eligible hierarchy claim for the requested level.
#[instrument(level = "info", skip_all)]
pub async fn sweep<N, C, R, O>(
    (nucl, repo, obj_dept): (&N, &R, &O),
    level: SubtreeSweepLevel,
) -> BaseRest<bool>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: SubtreeRepo<C> + PageArtworkRepo<C> + Send + Sync,
    O: ObjDept<PageArtworkImage, C>
        + ObjDept<ChapterArtwork, C>
        + ObjDept<PageSourceImage, C>
        + ObjDept<ComicCover, C>
        + ObjDept<TeamAvatar, C>
        + Send
        + Sync,
{
    let swept = nucl
        .coord(async |context| {
            //
            let target =
                ClaimSubtreeSweep { level }.step_on(repo, context).await?;

            let Some(target) = target else {
                return accept!(false);
            };

            match &target {
                //
                SubtreeDeleteSweepTarget::Chapter { id } => {
                    //
                    let artworks = ListPageArtworkInfos { chapter_id: id }
                        .step_on(repo, context)
                        .await?;

                    let artwork_ids = artworks
                        .into_iter()
                        .map(|info| info.id)
                        .collect::<Vec<_>>();

                    let page_artwork_image_ids =
                        PageArtworkImage::ids(&artwork_ids)?;

                    DeleteObjs::<PageArtworkImage>::new(
                        &page_artwork_image_ids,
                    )
                    .step_on(obj_dept, context)
                    .await
                    .map_err(BaseError::from)?;

                    let chapter_artwork_ids =
                        ChapterArtwork::ids(std::slice::from_ref(id))?;

                    DeleteObjs::<ChapterArtwork>::new(&chapter_artwork_ids)
                        .step_on(obj_dept, context)
                        .await
                        .map_err(BaseError::from)?;

                    let page_source_ids =
                        ListSubtreePageSourceIds { chapter_id: id }
                            .step_on(repo, context)
                            .await?;

                    let page_source_image_ids =
                        PageSourceImage::ids(&page_source_ids)?;

                    DeleteObjs::<PageSourceImage>::new(&page_source_image_ids)
                        .step_on(obj_dept, context)
                        .await
                        .map_err(BaseError::from)?;
                }

                SubtreeDeleteSweepTarget::Comics { ids } => {
                    //
                    let comic_cover_ids = ComicCover::ids(ids)?;

                    DeleteObjs::<ComicCover>::new(&comic_cover_ids)
                        .step_on(obj_dept, context)
                        .await
                        .map_err(BaseError::from)?;
                }

                SubtreeDeleteSweepTarget::Team { id } => {
                    //
                    let team_avatar_ids =
                        TeamAvatar::ids(std::slice::from_ref(id))?;

                    DeleteObjs::<TeamAvatar>::new(&team_avatar_ids)
                        .step_on(obj_dept, context)
                        .await
                        .map_err(BaseError::from)?;
                }

                SubtreeDeleteSweepTarget::Worksets { .. } => {}
            }

            SweepSubtree { target: &target }
                .step_on(repo, context)
                .await?;

            accept!(true)
        })
        .await?;

    accept!(swept)
}
