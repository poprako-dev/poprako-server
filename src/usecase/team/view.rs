//! Team presentation assembly.

use poprako_orchestra::Context;

use poprako_obj_dept::ObjDeptView;

use crate::data::view::team::TeamInfoView;
use crate::model::read::proj::team::TeamInfo;
use crate::part::obj_dept::TeamAvatar;
use crate::result::{BaseRest, accept};
use crate::usecase::internal::view::obj_urls::{
    ObjUrlBatch, ObjUrlViews, load_obj_urls,
};

/// Resolves one team model with its avatar origin and thumbnail URLs.
pub async fn team_info_view<C, O>(
    obj_dept: &O,
    model: TeamInfo,
) -> BaseRest<TeamInfoView>
where
    C: Context,
    O: ObjDeptView<TeamAvatar, C> + Sync,
{
    let team_ids = vec![model.id.as_str()];

    let mut avatar_urls = avatar_urls(obj_dept, team_ids).await?;

    let urls = avatar_urls.take(&model.id);

    accept(team_info_view_from_urls(model, urls))
}

/// Renders one team with URLs from an already-loaded object snapshot.
pub fn team_info_view_from_urls(
    model: TeamInfo,
    urls: Option<ObjUrlViews>,
) -> TeamInfoView {
    //
    let (avatar_url, avatar_thumbnail_url) = urls.unwrap_or_default();

    TeamInfoView::from_model(model, avatar_url, avatar_thumbnail_url)
}

/// Resolves team models with one avatar metadata query.
pub async fn team_info_views<C, O>(
    obj_dept: &O,
    models: Vec<TeamInfo>,
) -> BaseRest<Vec<TeamInfoView>>
where
    C: Context,
    O: ObjDeptView<TeamAvatar, C> + Sync,
{
    let team_ids = models.iter().map(|model| model.id.as_str()).collect();

    let mut avatar_urls = avatar_urls(obj_dept, team_ids).await?;

    accept(
        models
            .into_iter()
            .map(|model| {
                //
                let urls = avatar_urls.take(&model.id);

                team_info_view_from_urls(model, urls)
            })
            .collect(),
    )
}

/// Resolves current avatar URLs from one metadata query for the supplied team IDs.
pub async fn avatar_urls<C, O>(
    obj_dept: &O,
    team_ids: Vec<&str>,
) -> BaseRest<ObjUrlBatch>
where
    C: Context,
    O: ObjDeptView<TeamAvatar, C> + Sync,
{
    load_obj_urls(obj_dept, team_ids).await
}
