//! Member-invitation presentation assembly.

use poprako_orchestra::Context;

use poprako_obj_dept::ObjDeptView;

use crate::data::view::member_invitation::MemberInvitationInfoView;
use crate::model::read::proj::member_invitation::MemberInvitationInfo;
use crate::part::obj_dept::UserAvatar;
use crate::result::{BaseRest, accept};
use crate::usecase::user::view::{avatar_urls, user_info_view_from_urls};

/// Resolves invitation models with one invitor-avatar metadata query.
pub async fn member_invitation_info_views<C, O>(
    obj_dept: &O,
    models: Vec<MemberInvitationInfo>,
) -> BaseRest<Vec<MemberInvitationInfoView>>
where
    C: Context,
    O: ObjDeptView<UserAvatar, C> + Sync,
{
    let user_ids = models
        .iter()
        .filter_map(|model| {
            model.invitor.as_ref().map(|user_info| user_info.id.clone())
        })
        .collect::<Vec<_>>();

    let urls = avatar_urls::<C, O>(obj_dept, &user_ids).await?;

    accept(
        models
            .into_iter()
            .map(|mut model| {
                //
                let invitor = model.invitor.take().map(|user_info| {
                    //
                    let avatar_urls = urls.get(&user_info.id);

                    user_info_view_from_urls(user_info, avatar_urls)
                });

                MemberInvitationInfoView::from_model(model, invitor)
            })
            .collect(),
    )
}
