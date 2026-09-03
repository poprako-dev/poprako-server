//! Relational hierarchy sweep orchestration.

use poprako_orchestra::{AtLeast, Context, Nucl, OperStep as _};
use tracing::instrument;

use poprako_obj_dept::ObjDept;
use poprako_obj_dept::oper::DeleteObjs;

use crate::model::read::proj::subtree_delete::SubtreeDeleteSweepTarget;
use crate::part::nucl::ReptRead;
use crate::part::obj_dept::{ComicCover, PageImage, TeamAvatar};
use crate::part::repo::oper::subtree_delete::{
    ClaimSubtreeSweep, ListSubtreePageIds, SweepSubtree,
};
use crate::part::repo::subtree_delete::SubtreeRepo;
use crate::result::{BaseError, BaseRest, accept};

/// Sweeps at most one eligible hierarchy tombstone in one transaction.
#[instrument(level = "info", skip_all)]
pub async fn sweep_once<N, C, R, O>(
    (nucl, repo, obj_dept): (&N, &R, &O),
) -> BaseRest<bool>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: SubtreeRepo<C> + Send + Sync,
    O: ObjDept<PageImage, C>
        + ObjDept<ComicCover, C>
        + ObjDept<TeamAvatar, C>
        + Send
        + Sync,
{
    let swept = nucl
        .coord(async |context| {
            //
            let target = ClaimSubtreeSweep.step_on(repo, context).await?;

            let Some(target) = target else {
                return accept(false);
            };

            match &target {
                //
                SubtreeDeleteSweepTarget::Chapter { id } => {
                    //
                    let page_ids = ListSubtreePageIds { chapter_id: id }
                        .step_on(repo, context)
                        .await?;

                    DeleteObjs::<PageImage>::new(&page_ids)
                        .step_on(obj_dept, context)
                        .await
                        .map_err(BaseError::from)?;
                }

                SubtreeDeleteSweepTarget::Comic { id } => {
                    //
                    DeleteObjs::<ComicCover>::new(std::slice::from_ref(id))
                        .step_on(obj_dept, context)
                        .await
                        .map_err(BaseError::from)?;
                }

                SubtreeDeleteSweepTarget::Team { id } => {
                    //
                    DeleteObjs::<TeamAvatar>::new(std::slice::from_ref(id))
                        .step_on(obj_dept, context)
                        .await
                        .map_err(BaseError::from)?;
                }

                SubtreeDeleteSweepTarget::Workset { .. } => {}
            }

            SweepSubtree { target: &target }
                .step_on(repo, context)
                .await?;

            accept(true)
        })
        .await?;

    accept(swept)
}
