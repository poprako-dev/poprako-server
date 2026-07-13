//! OpenAPI documentation assembly.
//!
//! Handler-level `#[utoipa::path]` attributes specify full paths (including
//! the `/api/v1` prefix). Swagger UI and the OpenAPI JSON are mounted in debug
//! builds only, outside `/api/v1`.

use utoipa::OpenApi;

use crate::api::http::handler;
use crate::api::http::result::HttpError;

use crate::data::announcement::AnnouncementInfoVal;
use crate::data::announcement::CreateAnnouncementParams;
use crate::data::announcement::CreateAnnouncementPayload;
use crate::data::assignment::AssignmentInfoVal;
use crate::data::assignment::JoinChapterAssignmentParams;
use crate::data::assignment::UpdateAssignmentRolesParams;
use crate::data::assignment_invitation::AssignmentInvitationInfoVal;
use crate::data::assignment_invitation::CreateAssignmentInvitationParams;
use crate::data::assignment_invitation::CreateAssignmentInvitationPayload;
use crate::data::assignment_invitation::JoinAssignmentInvitationParams;
use crate::data::auth::LoginAuthParams;
use crate::data::auth::LoginAuthPayload;
use crate::data::auth::RegisterAuthParams;
use crate::data::auth::RegisterAuthPayload;
use crate::data::chapter::ChapterInfoVal;
use crate::data::chapter::CreateChapterParams;
use crate::data::chapter::CreateChapterPayload;
use crate::data::chapter::UpdateChapterInfoParams;
use crate::data::chapter::UpdateChapterStageParams;
use crate::data::chapter_port::ExportChapterTranslationPayload;
use crate::data::chapter_port::ImportChapterTranslationParams;
use crate::data::chapter_port::ImportChapterTranslationPayload;
use crate::data::comic::ComicInfoVal;
use crate::data::comic::CreateComicParams;
use crate::data::comic::CreateComicPayload;
use crate::data::comic::MarkComicCoverUploadedParams;
use crate::data::comic::ReserveComicCoverParams;
use crate::data::comic::ReserveComicCoverPayload;
use crate::data::comic::UpdateComicInfoParams;
use crate::data::comic_archive::ArchiveComicPayload;
use crate::data::comment::CommentInfoVal;
use crate::data::comment::CreateCommentParams;
use crate::data::comment::CreateCommentPayload;
use crate::data::member::CreateMemberParams;
use crate::data::member::CreateMemberPayload;
use crate::data::member::JoinTeamParams;
use crate::data::member::MemberInfoVal;
use crate::data::member::UpdateMemberRolesParams;
use crate::data::member_invitation::CreateMemberInvitationParams;
use crate::data::member_invitation::CreateMemberInvitationPayload;
use crate::data::member_invitation::MemberInvitationInfoVal;
use crate::data::member_invitation::UpdateMemberInvitationRolesParams;
use crate::data::page::MarkPageImageUploadedParams;
use crate::data::page::PageCreationPayload;
use crate::data::page::PageInfoVal;
use crate::data::page::ReserveChapterPagesParams;
use crate::data::page::ReserveChapterPagesPayload;
use crate::data::page::ReservePageImageParams;
use crate::data::page::ReservePageImagePayload;
use crate::data::system_mail::MarkSystemMailReadParams;
use crate::data::system_mail::SystemMailInfoVal;
use crate::data::team::CreateTeamParams;
use crate::data::team::MarkTeamAvatarUploadedParams;
use crate::data::team::ReserveTeamAvatarParams;
use crate::data::team::ReserveTeamAvatarPayload;
use crate::data::team::TeamInfoVal;
use crate::data::team::UpdateTeamInfoParams;
use crate::data::unit::ListPageUnitInfosPayload;
use crate::data::unit::SavePageUnitsParams;
use crate::data::unit::SavePageUnitsPayload;
use crate::data::unit::UnitDiffParams;
use crate::data::unit::UnitInfoVal;
use crate::data::unit::UnitOperParams;
use crate::data::user::MarkUserAvatarUploadedParams;
use crate::data::user::ReserveUserAvatarParams;
use crate::data::user::ReserveUserAvatarPayload;
use crate::data::user::UpdateUserInfoParams;
use crate::data::user::UserInfoVal;
use crate::data::workset::CreateWorksetParams;
use crate::data::workset::CreateWorksetPayload;
use crate::data::workset::UpdateWorksetInfoParams;
use crate::data::workset::WorksetInfoVal;
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
        RegisterAuthParams,
        RegisterAuthPayload,
        LoginAuthParams,
        LoginAuthPayload,
        UserInfoVal,
        UpdateUserInfoParams,
        ReserveUserAvatarParams,
        ReserveUserAvatarPayload,
        MarkUserAvatarUploadedParams,
        TeamInfoVal,
        CreateTeamParams,
        UpdateTeamInfoParams,
        ReserveTeamAvatarParams,
        ReserveTeamAvatarPayload,
        MarkTeamAvatarUploadedParams,
        WorksetInfoVal,
        CreateWorksetParams,
        CreateWorksetPayload,
        UpdateWorksetInfoParams,
        ComicInfoVal,
        CreateComicParams,
        CreateComicPayload,
        UpdateComicInfoParams,
        ReserveComicCoverParams,
        ReserveComicCoverPayload,
        MarkComicCoverUploadedParams,
        ArchiveComicPayload,
        ChapterInfoVal,
        CreateChapterParams,
        CreateChapterPayload,
        UpdateChapterInfoParams,
        UpdateChapterStageParams,
        ExportChapterTranslationPayload,
        ImportChapterTranslationParams,
        ImportChapterTranslationPayload,
        PageInfoVal,
        PageCreationPayload,
        ReserveChapterPagesParams,
        ReserveChapterPagesPayload,
        ReservePageImageParams,
        ReservePageImagePayload,
        MarkPageImageUploadedParams,
        UnitInfoVal,
        ListPageUnitInfosPayload,
        SavePageUnitsParams,
        SavePageUnitsPayload,
        UnitDiffParams,
        UnitOperParams,
        AssignmentInfoVal,
        JoinChapterAssignmentParams,
        UpdateAssignmentRolesParams,
        AssignmentInvitationInfoVal,
        CreateAssignmentInvitationParams,
        CreateAssignmentInvitationPayload,
        JoinAssignmentInvitationParams,
        SystemMailInfoVal,
        MarkSystemMailReadParams,
        AnnouncementInfoVal,
        CreateAnnouncementParams,
        CreateAnnouncementPayload,
        CommentInfoVal,
        CreateCommentParams,
        CreateCommentPayload,
        MemberInfoVal,
        CreateMemberParams,
        CreateMemberPayload,
        JoinTeamParams,
        UpdateMemberRolesParams,
        MemberInvitationInfoVal,
        CreateMemberInvitationParams,
        CreateMemberInvitationPayload,
        UpdateMemberInvitationRolesParams,
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
