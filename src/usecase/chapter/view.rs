//! Chapter presentation assembly.

use poprako_orchestra::Context;

use poprako_obj_dept::ObjDeptView;

use crate::data::view::chapter::ChapterInfoView;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::part::obj_dept::{ComicCover, TeamAvatar, UserAvatar};
use crate::result::{BaseRest, accept};
use crate::usecase::internal::view::{ObjViewIds, ObjViewSnapshot};

/// Resolves chapter models from one request-scoped object URL snapshot.
pub async fn chapter_info_views<C, O>(
    obj_dept: &O,
    models: Vec<ChapterInfo>,
) -> BaseRest<Vec<ChapterInfoView>>
where
    C: Context,
    O: ObjDeptView<ComicCover, C>
        + ObjDeptView<TeamAvatar, C>
        + ObjDeptView<UserAvatar, C>
        + Sync,
{
    let mut ids = ObjViewIds::default();

    ids.collect_chapters(&models);

    let snapshot = ObjViewSnapshot::load::<C, O>(obj_dept, ids).await?;

    accept(
        models
            .into_iter()
            .map(|model| snapshot.chapter(model))
            .collect(),
    )
}
