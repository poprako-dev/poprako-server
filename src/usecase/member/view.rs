//! Member presentation assembly.

use poprako_orchestra::Context;

use poprako_obj_dept::ObjDeptView;

use crate::data::view::member::MemberInfoView;
use crate::model::read::proj::member::MemberInfo;
use crate::part::obj_dept::{TeamAvatar, UserAvatar};
use crate::result::{BaseRest, accept};
use crate::usecase::team::view::{
    avatar_urls as team_avatar_urls, team_info_view_from_urls,
};
use crate::usecase::user::view::{
    avatar_urls as user_avatar_urls, user_info_view_from_urls,
};

/// Resolves one member model and its included user and team.
pub async fn member_info_view<C, O>(
    obj_dept: &O,
    mut model: MemberInfo,
) -> BaseRest<MemberInfoView>
where
    C: Context,
    O: ObjDeptView<TeamAvatar, C> + ObjDeptView<UserAvatar, C> + Sync,
{
    let user_ids = model
        .user
        .as_ref()
        .map(|user_info| user_info.id.as_str())
        .into_iter()
        .collect::<Vec<_>>();

    let team_ids = model
        .team
        .as_ref()
        .map(|team_info| team_info.id.as_str())
        .into_iter()
        .collect::<Vec<_>>();

    let (user_urls, team_urls) = futures_util::try_join!(
        user_avatar_urls::<C, O>(obj_dept, &user_ids),
        team_avatar_urls::<C, O>(obj_dept, &team_ids),
    )?;

    let user = model.user.take().map(|user_info| {
        //
        let avatar_urls = user_urls.get(&user_info.id);

        user_info_view_from_urls(user_info, avatar_urls)
    });

    let team = model.team.take().map(|team_info| {
        //
        let avatar_urls = team_urls.get(&team_info.id);

        team_info_view_from_urls(team_info, avatar_urls)
    });

    accept(MemberInfoView::from_model(model, user, team))
}

/// Resolves member models with one query per avatar object type.
pub async fn member_info_views<C, O>(
    obj_dept: &O,
    models: Vec<MemberInfo>,
) -> BaseRest<Vec<MemberInfoView>>
where
    C: Context,
    O: ObjDeptView<TeamAvatar, C> + ObjDeptView<UserAvatar, C> + Sync,
{
    let user_ids = models
        .iter()
        .filter_map(|model| {
            model.user.as_ref().map(|user_info| user_info.id.as_str())
        })
        .collect::<Vec<_>>();

    let team_ids = models
        .iter()
        .filter_map(|model| {
            model.team.as_ref().map(|team_info| team_info.id.as_str())
        })
        .collect::<Vec<_>>();

    let (user_urls, team_urls) = futures_util::try_join!(
        user_avatar_urls::<C, O>(obj_dept, &user_ids),
        team_avatar_urls::<C, O>(obj_dept, &team_ids),
    )?;

    accept(
        models
            .into_iter()
            .map(|mut model| {
                //
                let user = model.user.take().map(|user_info| {
                    //
                    let avatar_urls = user_urls.get(&user_info.id);

                    user_info_view_from_urls(user_info, avatar_urls)
                });

                let team = model.team.take().map(|team_info| {
                    //
                    let avatar_urls = team_urls.get(&team_info.id);

                    team_info_view_from_urls(team_info, avatar_urls)
                });

                MemberInfoView::from_model(model, user, team)
            })
            .collect(),
    )
}
