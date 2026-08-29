//! Use-case-side resolution of object-backed response views.

use poprako_orchestra::{OperRun as _, Run};

use poprako_obj_dept::obj_inst;
use poprako_obj_dept::oper::{GenObjUrl, GetObjMeta};
use poprako_obj_dept::rest::ObjDeptError;

use crate::data::view::announcement::AnnouncementInfoView;
use crate::data::view::assignment::AssignmentInfoView;
use crate::data::view::chapter::ChapterInfoView;
use crate::data::view::comic::ComicInfoView;
use crate::data::view::comment::CommentInfoView;
use crate::data::view::member::MemberInfoView;
use crate::data::view::member_invitation::MemberInvitationInfoView;
use crate::data::view::page::PageInfoView;
use crate::data::view::team::TeamInfoView;
use crate::data::view::user::UserInfoView;
use crate::model::read::proj::announcement::AnnouncementInfo;
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::comment::CommentInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::model::read::proj::member_invitation::MemberInvitationInfo;
use crate::model::read::proj::page::PageInfo;
use crate::model::read::proj::team::TeamInfo;
use crate::model::read::proj::user::UserInfo;
use crate::part::obj_dept::{ComicCover, PageImage, TeamAvatar, UserAvatar};
use crate::result::{BaseError, BaseRest, accept};

/// Resolves one user model and its avatar URL.
pub async fn user_info_view<O>(
    obj_dept: &O,
    model: UserInfo,
) -> BaseRest<UserInfoView>
where
    O: for<'a> Run<GenObjUrl<'a, UserAvatar>, Error = ObjDeptError> + Sync,
{
    let avatar_url = obj_inst! { GenObjUrl<UserAvatar> { id: &model.id } }
        .run_on(obj_dept)
        .await
        .map_err(BaseError::from)?;

    accept(UserInfoView::from_model(model, avatar_url.map(Into::into)))
}

/// Resolves one team model and its avatar URL.
pub async fn team_info_view<O>(
    obj_dept: &O,
    model: TeamInfo,
) -> BaseRest<TeamInfoView>
where
    O: for<'a> Run<GenObjUrl<'a, TeamAvatar>, Error = ObjDeptError> + Sync,
{
    let avatar_url = obj_inst! { GenObjUrl<TeamAvatar> { id: &model.id } }
        .run_on(obj_dept)
        .await
        .map_err(BaseError::from)?;

    accept(TeamInfoView::from_model(model, avatar_url.map(Into::into)))
}

/// Resolves one comic model and every included object-backed model.
pub async fn comic_info_view<O>(
    obj_dept: &O,
    mut model: ComicInfo,
) -> BaseRest<ComicInfoView>
where
    O: for<'a> Run<GenObjUrl<'a, ComicCover>, Error = ObjDeptError>
        + for<'a> Run<GenObjUrl<'a, TeamAvatar>, Error = ObjDeptError>
        + for<'a> Run<GenObjUrl<'a, UserAvatar>, Error = ObjDeptError>
        + Sync,
{
    let cover_url = obj_inst! { GenObjUrl<ComicCover> { id: &model.id } }
        .run_on(obj_dept)
        .await
        .map_err(BaseError::from)?;

    let team = match model.team.take() {
        //
        Some(team_info) => Some(team_info_view(obj_dept, team_info).await?),

        None => None,
    };

    let creator = match model.creator.take() {
        //
        Some(user_info) => Some(user_info_view(obj_dept, user_info).await?),

        None => None,
    };

    accept(ComicInfoView::from_model(
        model,
        cover_url.map(Into::into),
        team,
        creator,
    ))
}

/// Resolves one chapter model and its included models.
pub async fn chapter_info_view<O>(
    obj_dept: &O,
    mut model: ChapterInfo,
) -> BaseRest<ChapterInfoView>
where
    O: for<'a> Run<GenObjUrl<'a, ComicCover>, Error = ObjDeptError>
        + for<'a> Run<GenObjUrl<'a, TeamAvatar>, Error = ObjDeptError>
        + for<'a> Run<GenObjUrl<'a, UserAvatar>, Error = ObjDeptError>
        + Sync,
{
    let comic = match model.comic.take() {
        //
        Some(comic_info) => Some(comic_info_view(obj_dept, comic_info).await?),

        None => None,
    };

    let creator = match model.creator.take() {
        //
        Some(user_info) => Some(user_info_view(obj_dept, user_info).await?),

        None => None,
    };

    accept(ChapterInfoView::from_model(model, comic, creator))
}

/// Resolves one assignment model and its included models.
pub async fn assignment_info_view<O>(
    obj_dept: &O,
    mut model: AssignmentInfo,
) -> BaseRest<AssignmentInfoView>
where
    O: for<'a> Run<GenObjUrl<'a, ComicCover>, Error = ObjDeptError>
        + for<'a> Run<GenObjUrl<'a, TeamAvatar>, Error = ObjDeptError>
        + for<'a> Run<GenObjUrl<'a, UserAvatar>, Error = ObjDeptError>
        + Sync,
{
    let user = match model.user.take() {
        //
        Some(user_info) => Some(user_info_view(obj_dept, user_info).await?),

        None => None,
    };

    let chapter = match model.chapter.take() {
        //
        Some(chapter_info) => {
            Some(chapter_info_view(obj_dept, chapter_info).await?)
        }

        None => None,
    };

    accept(AssignmentInfoView::from_model(model, user, chapter))
}

/// Resolves one announcement model and its included author.
pub async fn announcement_info_view<O>(
    obj_dept: &O,
    mut model: AnnouncementInfo,
) -> BaseRest<AnnouncementInfoView>
where
    O: for<'a> Run<GenObjUrl<'a, UserAvatar>, Error = ObjDeptError> + Sync,
{
    let user = match model.user.take() {
        //
        Some(user_info) => Some(user_info_view(obj_dept, user_info).await?),

        None => None,
    };

    accept(AnnouncementInfoView::from_model(model, user))
}

/// Resolves one comment model and its included author.
pub async fn comment_info_view<O>(
    obj_dept: &O,
    mut model: CommentInfo,
) -> BaseRest<CommentInfoView>
where
    O: for<'a> Run<GenObjUrl<'a, UserAvatar>, Error = ObjDeptError> + Sync,
{
    let user = match model.user.take() {
        //
        Some(user_info) => Some(user_info_view(obj_dept, user_info).await?),

        None => None,
    };

    accept(CommentInfoView::from_model(model, user))
}

/// Resolves one member model and its included user and team.
pub async fn member_info_view<O>(
    obj_dept: &O,
    mut model: MemberInfo,
) -> BaseRest<MemberInfoView>
where
    O: for<'a> Run<GenObjUrl<'a, TeamAvatar>, Error = ObjDeptError>
        + for<'a> Run<GenObjUrl<'a, UserAvatar>, Error = ObjDeptError>
        + Sync,
{
    let user = match model.user.take() {
        //
        Some(user_info) => Some(user_info_view(obj_dept, user_info).await?),

        None => None,
    };

    let team = match model.team.take() {
        //
        Some(team_info) => Some(team_info_view(obj_dept, team_info).await?),

        None => None,
    };

    accept(MemberInfoView::from_model(model, user, team))
}

/// Resolves one member invitation and its included invitor.
pub async fn member_invitation_info_view<O>(
    obj_dept: &O,
    mut model: MemberInvitationInfo,
) -> BaseRest<MemberInvitationInfoView>
where
    O: for<'a> Run<GenObjUrl<'a, UserAvatar>, Error = ObjDeptError> + Sync,
{
    let invitor = match model.invitor.take() {
        //
        Some(user_info) => Some(user_info_view(obj_dept, user_info).await?),

        None => None,
    };

    accept(MemberInvitationInfoView::from_model(model, invitor))
}

/// Resolves one page model and its image metadata and URL.
pub async fn page_info_view<O>(
    obj_dept: &O,
    model: PageInfo,
) -> BaseRest<PageInfoView>
where
    O: for<'a> Run<GetObjMeta<'a, PageImage>, Error = ObjDeptError>
        + for<'a> Run<GenObjUrl<'a, PageImage>, Error = ObjDeptError>
        + Sync,
{
    let obj_meta = obj_inst! { GetObjMeta<PageImage> { id: &model.id } }
        .run_on(obj_dept)
        .await
        .map_err(BaseError::from)?;

    let image_url = obj_inst! { GenObjUrl<PageImage> { id: &model.id } }
        .run_on(obj_dept)
        .await
        .map_err(BaseError::from)?;

    accept(PageInfoView::from_model(
        model,
        obj_meta.as_ref(),
        image_url.map(Into::into),
    ))
}
