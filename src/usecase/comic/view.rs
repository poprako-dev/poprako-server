//! Comic presentation assembly.

use poprako_orchestra::Context;

use poprako_obj_dept::ObjDeptView;

use crate::data::view::comic::ComicInfoView;
use crate::model::read::proj::comic::ComicInfo;
use crate::part::obj_dept::{ComicCover, TeamAvatar, UserAvatar};
use crate::result::{BaseRest, accept};
use crate::usecase::internal::view::{ObjViewIds, ObjViewSnapshot};

/// Resolves one comic model and every included object-backed model.
pub async fn comic_info_view<C, O>(
    obj_dept: &O,
    model: ComicInfo,
) -> BaseRest<ComicInfoView>
where
    C: Context,
    O: ObjDeptView<ComicCover, C>
        + ObjDeptView<TeamAvatar, C>
        + ObjDeptView<UserAvatar, C>
        + Sync,
{
    let mut ids = ObjViewIds::default();

    ids.collect_comics(std::slice::from_ref(&model));

    let snapshot = ObjViewSnapshot::load::<C, O>(obj_dept, ids).await?;

    accept(snapshot.comic(model))
}
