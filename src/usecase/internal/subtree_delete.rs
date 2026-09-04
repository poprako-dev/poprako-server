//! Shared Chapter object cleanup inside caller-owned transactions.

use poprako_orchestra::{Context, OperStep as _};

use poprako_obj_dept::ObjDept;
use poprako_obj_dept::oper::DeleteObjs;

use crate::model::read::proj::subtree_delete::SubtreeDeleteScope;
use crate::part::obj_dept::PageImage;
use crate::part::repo::oper::subtree_delete::ListSubtreePageIds;
use crate::part::repo::subtree_delete::SubtreeRepo;
use crate::result::{BaseError, BaseRest};

/// Enqueues all Page image deletions for one locked Chapter.
pub async fn delete_page_objs<C, R, O>(
    repo: &R,
    obj_dept: &O,
    context: &mut C,
    scope: &SubtreeDeleteScope,
) -> BaseRest<()>
where
    C: Context,
    R: SubtreeRepo<C> + Sync,
    O: ObjDept<PageImage, C> + Sync,
{
    let SubtreeDeleteScope::Chapter { chapter_id, .. } = scope else {
        //
        return Err(BaseError::Unrecoverable {
            message: "page object cleanup requires one Chapter scope".into(),
        });
    };

    let page_ids = ListSubtreePageIds { chapter_id }
        .step_on(repo, context)
        .await?;

    DeleteObjs::<PageImage>::new(&page_ids)
        .step_on(obj_dept, context)
        .await
        .map_err(BaseError::from)
}
