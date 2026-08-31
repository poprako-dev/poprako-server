//! Announcement presentation assembly.

use poprako_orchestra::Context;

use poprako_obj_dept::ObjDeptView;

use crate::data::view::announcement::AnnouncementInfoView;
use crate::model::read::proj::announcement::AnnouncementInfo;
use crate::part::obj_dept::UserAvatar;
use crate::result::{BaseRest, accept};
use crate::usecase::user::view::{avatar_urls, user_info_view_from_urls};

/// Resolves announcement models with one author-avatar metadata query.
pub async fn announcement_info_views<C, O>(
    obj_dept: &O,
    models: Vec<AnnouncementInfo>,
) -> BaseRest<Vec<AnnouncementInfoView>>
where
    C: Context,
    O: ObjDeptView<UserAvatar, C> + Sync,
{
    let user_ids = models
        .iter()
        .filter_map(|model| {
            model.user.as_ref().map(|user_info| user_info.id.clone())
        })
        .collect::<Vec<_>>();

    let urls = avatar_urls::<C, O>(obj_dept, &user_ids).await?;

    accept(
        models
            .into_iter()
            .map(|mut model| {
                //
                let user = model.user.take().map(|user_info| {
                    //
                    let avatar_urls = urls.get(&user_info.id);

                    user_info_view_from_urls(user_info, avatar_urls)
                });

                AnnouncementInfoView::from_model(model, user)
            })
            .collect(),
    )
}
