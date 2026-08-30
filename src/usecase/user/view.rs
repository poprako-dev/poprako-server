//! User presentation assembly.

use std::collections::HashMap;

use poprako_orchestra::{Context, OperRun as _};

use poprako_obj_dept::ObjDeptView;
use poprako_obj_dept::model::url::ObjUrls;
use poprako_obj_dept::oper::{GenObjUrls, ListObjMetas};

use crate::data::view::user::UserInfoView;
use crate::model::read::proj::user::UserInfo;
use crate::part::obj_dept::UserAvatar;
use crate::result::{BaseError, BaseRest, accept};

/// Resolves one user model with its avatar origin and thumbnail URLs.
pub async fn user_info_view<C, O>(
    obj_dept: &O,
    model: UserInfo,
) -> BaseRest<UserInfoView>
where
    C: Context,
    O: ObjDeptView<UserAvatar, C> + Sync,
{
    let avatar_urls =
        avatar_urls::<C, O>(obj_dept, std::slice::from_ref(&model.id)).await?;

    let urls = avatar_urls.get(&model.id);

    accept(user_info_view_from_urls(model, urls))
}

/// Renders one user with URLs from an already-loaded object snapshot.
pub fn user_info_view_from_urls(
    model: UserInfo,
    urls: Option<&ObjUrls>,
) -> UserInfoView {
    //
    UserInfoView::from_model(
        model,
        urls.map(|urls| urls.origin_url.to_string()),
        urls.and_then(|urls| urls.thumbnail_url.as_ref())
            .map(ToString::to_string),
    )
}

/// Resolves current avatar URLs from one metadata query for the supplied user IDs.
pub async fn avatar_urls<C, O>(
    obj_dept: &O,
    user_ids: &[String],
) -> BaseRest<HashMap<String, ObjUrls>>
where
    C: Context,
    O: ObjDeptView<UserAvatar, C> + Sync,
{
    if user_ids.is_empty() {
        return accept(HashMap::new());
    }

    let mut user_ids = user_ids.to_vec();

    user_ids.sort_unstable();

    user_ids.dedup();

    let obj_metas = ListObjMetas::<UserAvatar>::new(&user_ids)
        .run_on(obj_dept)
        .await
        .map_err(BaseError::from)?;

    let avatar_urls = GenObjUrls::<UserAvatar>::new(&obj_metas)
        .run_on(obj_dept)
        .await
        .map_err(BaseError::from)?;

    accept(avatar_urls)
}
