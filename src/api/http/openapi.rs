//! OpenAPI documentation assembly.
//!
//! Handler-level `#[utoipa::path]` attributes specify full paths (including
//! the `/api/v1` prefix). Swagger UI and the OpenAPI JSON are mounted in debug
//! builds only, outside `/api/v1`.

use utoipa::OpenApi;

use crate::api::http::handler;
use crate::api::http::result::HttpError;
use crate::data::announcement::{
    AnnouncementInfoVal, CreateAnnouncementParams, CreateAnnouncementPayload,
};
use crate::data::assignment::{
    AssignmentInfoVal, JoinChapterAssignmentParams, UpdateAssignmentRolesParams,
};
use crate::data::assignment_invitation::{
    AssignmentInvitationInfoVal, CreateAssignmentInvitationParams,
    CreateAssignmentInvitationPayload, JoinAssignmentInvitationParams,
};
use crate::data::auth::{
    LoginAuthParams, LoginAuthPayload, RegisterAuthParams, RegisterAuthPayload,
};
use crate::data::chapter::{
    ChapterInfoVal, CreateChapterParams, CreateChapterPayload,
    UpdateChapterInfoParams, UpdateChapterStageParams,
};
use crate::data::chapter_port::{
    ExportChapterTranslationPayload, ImportChapterTranslationParams,
    ImportChapterTranslationPayload,
};
use crate::data::comic::{
    ComicInfoVal, CreateComicParams, CreateComicPayload,
    MarkComicCoverUploadedParams, ReserveComicCoverParams,
    ReserveComicCoverPayload, UpdateComicInfoParams,
};
use crate::data::comic_archive::ArchiveComicPayload;
use crate::data::comic_list::ListComicInfosPayload;
use crate::data::comment::{
    CommentInfoVal, CreateCommentParams, CreateCommentPayload,
};
use crate::data::image::ImageUploadSlotVal;
use crate::data::member::{
    CreateMemberParams, CreateMemberPayload, JoinTeamParams, MemberInfoVal,
    UpdateMemberRolesParams,
};
use crate::data::member_invitation::{
    CreateMemberInvitationParams, CreateMemberInvitationPayload,
    MemberInvitationInfoVal, UpdateMemberInvitationRolesParams,
};
use crate::data::page::{
    MarkPageImageUploadedParams, PageImageParams, PageInfoVal,
    ReserveChapterPagesParams, ReserveChapterPagesPayload,
    ReservePageImageParams, ReservedPagePayload,
};
use crate::data::system_mail::{MarkSystemMailReadParams, SystemMailInfoVal};
use crate::data::team::{
    CreateTeamParams, MarkTeamAvatarUploadedParams, ReserveTeamAvatarParams,
    ReserveTeamAvatarPayload, TeamInfoVal, UpdateTeamInfoParams,
};
use crate::data::term::{
    CreateTermParams, CreateTermPayload, TermInfoVal, UpdateTermInfoParams,
};
use crate::data::termbase::{
    CreateTermbaseParams, CreateTermbasePayload, TermbaseInfoVal,
    UpdateTermbaseInfoParams,
};
use crate::data::unit::{
    ListPageUnitInfosPayload, SavePageUnitsParams, SavePageUnitsPayload,
    UnitDiffParams, UnitInfoVal, UnitOperParams,
};
use crate::data::user::{
    MarkUserAvatarUploadedParams, ReserveUserAvatarParams,
    ReserveUserAvatarPayload, UpdateUserInfoParams, UpdateUserPasswordParams,
    UserInfoVal,
};
use crate::data::workset::{
    CreateWorksetParams, CreateWorksetPayload, UpdateWorksetInfoParams,
    WorksetInfoVal,
};
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
        handler::health::detailed_metrics,
        handler::auth::register,
        handler::auth::login,
        handler::auth::logout,
        handler::user::get_my_info,
        handler::user::get_info,
        handler::user::update_info,
        handler::user::update_password,
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
        handler::comic::export_archives,
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
        handler::termbase::create,
        handler::termbase::list_team_infos,
        handler::termbase::list_comic_infos,
        handler::termbase::get_info,
        handler::termbase::update_info,
        handler::termbase::delete,
        handler::term::create,
        handler::term::list_infos,
        handler::term::get_info,
        handler::term::update_info,
        handler::term::delete,
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
        UpdateUserPasswordParams,
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
        ListComicInfosPayload,
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
        PageImageParams,
        ImageUploadSlotVal,
        ReserveChapterPagesParams,
        ReserveChapterPagesPayload,
        ReservePageImageParams,
        ReservedPagePayload,
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
        TermbaseInfoVal,
        CreateTermbaseParams,
        CreateTermbasePayload,
        UpdateTermbaseInfoParams,
        TermInfoVal,
        CreateTermParams,
        CreateTermPayload,
        UpdateTermInfoParams,
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
