//! OpenAPI documentation assembly.
//!
//! Handler-level `#[utoipa::path]` attributes specify full paths (including
//! the `/api/v1` prefix). Swagger UI and the OpenAPI JSON are mounted in debug
//! builds only, outside `/api/v1`.

use utoipa::OpenApi;

use crate::api::http::handler;
use crate::api::http::result::HttpError;
use crate::data::announcement_data;
use crate::data::assignment_data;
use crate::data::assignment_invitation_data;
use crate::data::auth_data;
use crate::data::chapter_data;
use crate::data::chapter_port_data;
use crate::data::comic_archive_data;
use crate::data::comic_data;
use crate::data::comment_data;
use crate::data::member_data;
use crate::data::member_invitation_data;
use crate::data::page_data;
use crate::data::system_mail_data;
use crate::data::team_data;
use crate::data::unit_data;
use crate::data::user_data;
use crate::data::workset_data;
use crate::value::announcement::AnnouncementInclOpt;
use crate::value::assignment::AssignmentInclOpt;
use crate::value::chapter::ChapterInclOpt;
use crate::value::comic::{ComicInclOpt, ComicWithOpt};
use crate::value::comment::CommentInclOpt;
use crate::value::member::MemberInclOpt;
use crate::value::member_invitation::MemberInvitationInclOpt;
use crate::value::role::RoleField;

/// Top-level OpenAPI document for the PopRaKo HTTP API.
#[derive(OpenApi)]
#[openapi(
    paths(
        handler::health::check_health,
        handler::auth::register,
        handler::auth::login,
        handler::auth::logout,
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
        handler::comic::archive,
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
        handler::chapter_port::export_download,
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
        handler::member::update_roles,
        handler::member::delete,
        handler::member::join,
        handler::member_invitation::create,
        handler::member_invitation::list_infos,
        handler::member_invitation::update_roles,
        handler::member_invitation::delete,
    ),
    components(schemas(
        HttpError,
        auth_data::RegisterData,
        auth_data::RegisterVal,
        auth_data::LoginData,
        auth_data::LoginVal,
        user_data::InfoVal,
        user_data::UpdateInfoData,
        user_data::ReserveAvatarData,
        user_data::ReserveAvatarVal,
        user_data::MarkAvatarUploadedData,
        team_data::InfoVal,
        team_data::CreateData,
        team_data::UpdateInfoData,
        team_data::ReserveAvatarData,
        team_data::ReserveAvatarVal,
        team_data::MarkAvatarUploadedData,
        workset_data::InfoVal,
        workset_data::CreateData,
        workset_data::CreateVal,
        workset_data::UpdateInfoData,
        comic_data::InfoVal,
        comic_data::CreateData,
        comic_data::CreateVal,
        comic_data::UpdateInfoData,
        comic_data::ReserveCoverData,
        comic_data::ReserveCoverVal,
        comic_data::MarkCoverUploadedData,
        comic_archive_data::Val,
        chapter_data::InfoVal,
        chapter_data::CreateData,
        chapter_data::CreateVal,
        chapter_data::PatchInfoData,
        chapter_data::UpdateStageData,
        chapter_port_data::TranslationExportVal,
        chapter_port_data::TranslationImportData,
        chapter_port_data::TranslationImportVal,
        page_data::InfoVal,
        page_data::CreationVal,
        page_data::ReserveChapterData,
        page_data::ReserveChapterVal,
        page_data::ReserveImageData,
        page_data::ReserveImageVal,
        page_data::MarkImageUploadedData,
        unit_data::InfoVal,
        unit_data::ListPageInfosVal,
        unit_data::SavePageData,
        unit_data::SavePageVal,
        unit_data::DiffData,
        OperData,
        assignment_data::InfoVal,
        assignment_data::JoinChapterData,
        assignment_data::UpdateRolesData,
        assignment_invitation_data::InfoVal,
        assignment_invitation_data::CreateData,
        assignment_invitation_data::CreateVal,
        assignment_invitation_data::JoinData,
        system_mail_data::Val,
        system_mail_data::MarkReadData,
        announcement_data::InfoVal,
        announcement_data::CreateData,
        announcement_data::CreateVal,
        comment_data::InfoVal,
        comment_data::CreateData,
        comment_data::CreateVal,
        member_data::InfoVal,
        member_data::CreateData,
        member_data::CreateVal,
        member_data::JoinTeamData,
        member_data::UpdateRolesData,
        member_invitation_data::InfoVal,
        member_invitation_data::CreateData,
        member_invitation_data::CreateVal,
        member_invitation_data::UpdateRolesData,
        ComicInclOpt,
        ComicWithOpt,
        ChapterInclOpt,
        MemberInclOpt,
        MemberInvitationInclOpt,
        AssignmentInclOpt,
        AnnouncementInclOpt,
        CommentInclOpt,
        RoleField,
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
