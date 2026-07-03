//! OpenAPI documentation assembly.
//!
//! Handler-level `#[utoipa::path]` attributes specify full paths (including
//! the `/api/v1` prefix). Swagger UI and the OpenAPI JSON are mounted in debug
//! builds only, outside `/api/v1`.

use utoipa::OpenApi;

use crate::api::http::handler;
use crate::api::http::result::HttpError;
use crate::data::announcement::{
    AnnouncementInfoVal, CreateAnnouncementData, CreateAnnouncementVal,
};
use crate::data::assignment::{AssignmentInfoVal, JoinChapterData, UpdateAssignmentRoleData};
use crate::data::assignment_invitation::{
    AssignmentInvitationInfoVal, CreateAssignmentInvitationData, CreateAssignmentInvitationVal,
    JoinAssignmentInvitationData,
};
use crate::data::auth::{LoginData, LoginVal, RegisterData, RegisterVal};
use crate::data::chapter::{
    ChapterInfoVal, CreateChapterData, CreateChapterVal, PatchChapterInfoData,
    UpdateChapterStageData,
};
use crate::data::chapter_port::{ChapterTranslationImportData, ChapterTranslationImportVal};
use crate::data::comic::{
    ComicInfoVal, CreateComicData, CreateComicVal, MarkComicCompletedData,
    MarkComicCoverUploadedData, ReserveComicCoverData, ReserveComicCoverVal, UpdateComicInfoData,
};
use crate::data::comment::{CommentInfoVal, CreateCommentData, CreateCommentVal};
use crate::data::member::{
    CreateMemberData, CreateMemberVal, JoinTeamData, MemberInfoVal, UpdateMemberRoleData,
};
use crate::data::member_invitation::{
    CreateMemberInvitationData, CreateMemberInvitationVal, MemberInvitationInfoVal,
    UpdateMemberInvitationInfoData,
};
use crate::data::page::{
    MarkPageImageUploadedData, PageCreationVal, PageInfoVal, ReserveChapterPagesData,
    ReserveChapterPagesVal, ReservePageImageData, ReservePageImageVal,
};
use crate::data::system_mail::{MarkSystemMailsReadData, SystemMailVal};
use crate::data::team::{
    CreateTeamData, MarkTeamAvatarUploadedData, ReserveTeamAvatarData, ReserveTeamAvatarVal,
    TeamInfoVal, UpdateTeamInfoData,
};
use crate::data::unit::{
    ListPageUnitInfosVal, SavePageUnitsData, SavePageUnitsVal, UnitDiffData, UnitInfoVal,
    UnitOperData,
};
use crate::data::user::{
    MarkUserAvatarUploadedData, ReserveUserAvatarData, ReserveUserAvatarVal, UpdateUserInfoData,
    UserInfoVal,
};
use crate::data::workset::{
    CreateWorksetData, CreateWorksetVal, UpdateWorksetInfoData, WorksetInfoVal,
};

/// Top-level OpenAPI document for the PopRaKo HTTP API.
#[derive(OpenApi)]
#[openapi(
    paths(
        handler::health::check_health,
        handler::auth::register,
        handler::auth::login,
        handler::user::get_my_info,
        handler::user::get_info,
        handler::user::update_info,
        handler::user::delete,
        handler::user::reserve_avatar,
        handler::user::mark_avatar_uploaded,
        handler::team::create,
        handler::team::list_infos,
        handler::team::get_info,
        handler::team::update_info,
        handler::team::reserve_avatar,
        handler::team::mark_avatar_uploaded,
        handler::team::delete,
        handler::workset::create,
        handler::workset::list_infos,
        handler::workset::get_info,
        handler::workset::update_info,
        handler::workset::delete,
        handler::comic::create,
        handler::comic::list_infos,
        handler::comic::get_info,
        handler::comic::update_info,
        handler::comic::reserve_cover,
        handler::comic::mark_cover_uploaded,
        handler::comic::mark_completed,
        handler::comic::delete,
        handler::chapter::create,
        handler::chapter::list_infos,
        handler::chapter::get_pinned,
        handler::chapter::get_info,
        handler::chapter::update_info,
        handler::chapter::advance_stage,
        handler::chapter::delete,
        handler::chapter_port::import,
        handler::chapter_port::export,
        handler::page::list_infos,
        handler::page::delete,
        handler::page::reserve_chapter_pages,
        handler::page::reserve_image,
        handler::page::mark_image_uploaded,
        handler::unit::list_infos,
        handler::unit::save_infos,
        handler::assignment::list_infos,
        handler::assignment::update_roles,
        handler::assignment::delete,
        handler::assignment::join,
        handler::assignment_invitation::create,
        handler::assignment_invitation::list_infos,
        handler::assignment_invitation::delete,
        handler::assignment_invitation::join,
        handler::system_mail::list_infos,
        handler::system_mail::mark_read,
        handler::announcement::create,
        handler::announcement::list_infos,
        handler::comment::create,
        handler::comment::list_infos,
        handler::member::create,
        handler::member::list_infos,
        handler::member::list_my_infos,
        handler::member::update_role,
        handler::member::delete,
        handler::member::join,
        handler::member_invitation::create,
        handler::member_invitation::list_infos,
        handler::member_invitation::update_info,
        handler::member_invitation::delete,
    ),
    components(schemas(
        HttpError,
        RegisterData,
        RegisterVal,
        LoginData,
        LoginVal,
        UserInfoVal,
        UpdateUserInfoData,
        ReserveUserAvatarData,
        ReserveUserAvatarVal,
        MarkUserAvatarUploadedData,
        TeamInfoVal,
        CreateTeamData,
        UpdateTeamInfoData,
        ReserveTeamAvatarData,
        ReserveTeamAvatarVal,
        MarkTeamAvatarUploadedData,
        WorksetInfoVal,
        CreateWorksetData,
        CreateWorksetVal,
        UpdateWorksetInfoData,
        ComicInfoVal,
        CreateComicData,
        CreateComicVal,
        UpdateComicInfoData,
        ReserveComicCoverData,
        ReserveComicCoverVal,
        MarkComicCoverUploadedData,
        MarkComicCompletedData,
        ChapterInfoVal,
        CreateChapterData,
        CreateChapterVal,
        PatchChapterInfoData,
        UpdateChapterStageData,
        ChapterTranslationImportData,
        ChapterTranslationImportVal,
        PageInfoVal,
        PageCreationVal,
        ReserveChapterPagesData,
        ReserveChapterPagesVal,
        ReservePageImageData,
        ReservePageImageVal,
        MarkPageImageUploadedData,
        UnitInfoVal,
        ListPageUnitInfosVal,
        SavePageUnitsData,
        SavePageUnitsVal,
        UnitDiffData,
        UnitOperData,
        AssignmentInfoVal,
        JoinChapterData,
        UpdateAssignmentRoleData,
        AssignmentInvitationInfoVal,
        CreateAssignmentInvitationData,
        CreateAssignmentInvitationVal,
        JoinAssignmentInvitationData,
        SystemMailVal,
        MarkSystemMailsReadData,
        AnnouncementInfoVal,
        CreateAnnouncementData,
        CreateAnnouncementVal,
        CommentInfoVal,
        CreateCommentData,
        CreateCommentVal,
        MemberInfoVal,
        CreateMemberData,
        CreateMemberVal,
        JoinTeamData,
        UpdateMemberRoleData,
        MemberInvitationInfoVal,
        CreateMemberInvitationData,
        CreateMemberInvitationVal,
        UpdateMemberInvitationInfoData,
    )),
    tags(
        (name = "health", description = "Health-check endpoints"),
        (name = "auth", description = "Authentication endpoints"),
        (name = "users", description = "User management endpoints"),
        (name = "teams", description = "Team management endpoints"),
        (name = "worksets", description = "Workset management endpoints"),
        (name = "comics", description = "Comic management endpoints"),
        (name = "chapters", description = "Chapter management endpoints"),
        (name = "chapter-port", description = "Chapter translation import/export endpoints"),
        (name = "pages", description = "Page management endpoints"),
        (name = "units", description = "Page unit endpoints"),
        (name = "members", description = "Member management endpoints"),
        (
            name = "member-invitations",
            description = "Member invitation endpoints",
        ),
        (name = "assignments", description = "Assignment endpoints"),
        (
            name = "assignment-invitations",
            description = "Assignment invitation endpoints",
        ),
        (name = "system-mails", description = "System mail endpoints"),
        (name = "announcements", description = "Announcement endpoints"),
        (name = "comments", description = "Comment endpoints"),
    )
)]
pub struct ApiDoc;
