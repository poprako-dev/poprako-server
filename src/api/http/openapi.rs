//! OpenAPI documentation assembly.
//!
//! Handler-level `#[utoipa::path]` attributes specify full paths (including
//! the `/api/v1` prefix). Swagger UI and the OpenAPI JSON are mounted in debug
//! builds only, outside `/api/v1`.

use utoipa::OpenApi;

use crate::api::http::handler;
use crate::api::http::result::HttpError;
use crate::data::instr::announcement::CreateAnnouncementInstr;
use crate::data::instr::assignment::{
    JoinChapterAssignmentInstr, UpdateAssignmentRolesInstr,
};
use crate::data::instr::assignment_invitation::{
    CreateAssignmentInvitationInstr, JoinAssignmentInvitationInstr,
};
use crate::data::instr::auth::{LoginAuthInstr, RegisterAuthInstr};
use crate::data::instr::chapter::{
    CreateChapterInstr, UpdateChapterInfoInstr, UpdateChapterStageInstr,
};
use crate::data::instr::chapter_port::ImportChapterTranslationInstr;
use crate::data::instr::comic::{
    CreateComicInstr, MarkComicCoverUploadedInstr, ReserveComicCoverInstr,
    UpdateComicInfoInstr,
};
use crate::data::instr::comment::CreateCommentInstr;
use crate::data::instr::member::{
    CreateMemberInstr, JoinTeamInstr, UpdateMemberRolesInstr,
};
use crate::data::instr::member_invitation::{
    CreateMemberInvitationInstr, UpdateMemberInvitationRolesInstr,
};
use crate::data::instr::page::{
    MarkPageImageUploadedInstr, PageImageInstr, ReserveChapterPagesInstr,
    ReservePageImageInstr,
};
use crate::data::instr::system_mail::MarkSystemMailReadInstr;
use crate::data::instr::team::{
    CreateTeamInstr, MarkTeamAvatarUploadedInstr, ReserveTeamAvatarInstr,
    UpdateTeamInfoInstr,
};
use crate::data::instr::term::{CreateTermInstr, UpdateTermInfoInstr};
use crate::data::instr::termbase::{
    CreateTermbaseInstr, UpdateTermbaseInfoInstr,
};
use crate::data::instr::unit::{
    UnitCoordInstr, UnitEditInstr, UnitRevisionInstr, UnitTranslationInstr,
};
use crate::data::instr::user::{
    MarkUserAvatarUploadedInstr, ReserveUserAvatarInstr, UpdateUserInfoInstr,
    UpdateUserPasswordInstr,
};
use crate::data::instr::workset::{CreateWorksetInstr, UpdateWorksetInfoInstr};
use crate::data::val::announcement::{
    AnnouncementInfoVal, CreateAnnouncementVal,
};
use crate::data::val::assignment::AssignmentInfoVal;
use crate::data::val::assignment_invitation::{
    AssignmentInvitationInfoVal, CreateAssignmentInvitationVal,
};
use crate::data::val::auth::{LoginAuthVal, RegisterAuthVal};
use crate::data::val::chapter::{ChapterInfoVal, CreateChapterVal};
use crate::data::val::chapter_port::{
    ExportChapterTranslationVal, ImportChapterTranslationVal,
};
use crate::data::val::comic::{
    ComicInfoVal, CreateComicVal, ReserveComicCoverVal,
};
use crate::data::val::comic_archive::ArchiveComicVal;
use crate::data::val::comic_list::ListComicInfosVal;
use crate::data::val::comment::{CommentInfoVal, CreateCommentVal};
use crate::data::val::member::{CreateMemberVal, MemberInfoVal};
use crate::data::val::member_invitation::{
    CreateMemberInvitationVal, MemberInvitationInfoVal,
};
use crate::data::val::page::{
    PageInfoVal, ReserveChapterPagesVal, ReservedPageVal,
};
use crate::data::val::system_mail::SystemMailInfoVal;
use crate::data::val::team::{ReserveTeamAvatarVal, TeamInfoVal};
use crate::data::val::term::{CreateTermVal, TermInfoVal};
use crate::data::val::termbase::{CreateTermbaseVal, TermbaseInfoVal};
use crate::data::val::unit::ListPageUnitInfosVal;
use crate::data::val::user::{ReserveUserAvatarVal, UserInfoVal};
use crate::data::val::workset::{CreateWorksetVal, WorksetInfoVal};
use crate::data::view::image::ImageUploadSlotView;
use crate::data::view::unit::UnitInfoView;
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
        handler::chapter::mark_pinned,
        handler::chapter::advance_stage,
        handler::chapter::delete,
        handler::chapter_port::import,
        handler::chapter_port::export,
        handler::chapter_port::export_download,
        handler::page::list_infos,
        handler::page::get_info,
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
        RegisterAuthInstr,
        RegisterAuthVal,
        LoginAuthInstr,
        LoginAuthVal,
        UserInfoVal,
        UpdateUserInfoInstr,
        UpdateUserPasswordInstr,
        ReserveUserAvatarInstr,
        ReserveUserAvatarVal,
        MarkUserAvatarUploadedInstr,
        TeamInfoVal,
        CreateTeamInstr,
        UpdateTeamInfoInstr,
        ReserveTeamAvatarInstr,
        ReserveTeamAvatarVal,
        MarkTeamAvatarUploadedInstr,
        WorksetInfoVal,
        CreateWorksetInstr,
        CreateWorksetVal,
        UpdateWorksetInfoInstr,
        ComicInfoVal,
        ListComicInfosVal,
        CreateComicInstr,
        CreateComicVal,
        UpdateComicInfoInstr,
        ReserveComicCoverInstr,
        ReserveComicCoverVal,
        MarkComicCoverUploadedInstr,
        ArchiveComicVal,
        ChapterInfoVal,
        CreateChapterInstr,
        CreateChapterVal,
        UpdateChapterInfoInstr,
        UpdateChapterStageInstr,
        ExportChapterTranslationVal,
        ImportChapterTranslationInstr,
        ImportChapterTranslationVal,
        PageInfoVal,
        PageImageInstr,
        ImageUploadSlotView,
        ReserveChapterPagesInstr,
        ReserveChapterPagesVal,
        ReservePageImageInstr,
        ReservedPageVal,
        MarkPageImageUploadedInstr,
        UnitInfoView,
        ListPageUnitInfosVal,
        UnitEditInstr,
        UnitCoordInstr,
        UnitTranslationInstr,
        UnitRevisionInstr,
        AssignmentInfoVal,
        JoinChapterAssignmentInstr,
        UpdateAssignmentRolesInstr,
        AssignmentInvitationInfoVal,
        CreateAssignmentInvitationInstr,
        CreateAssignmentInvitationVal,
        JoinAssignmentInvitationInstr,
        SystemMailInfoVal,
        MarkSystemMailReadInstr,
        AnnouncementInfoVal,
        CreateAnnouncementInstr,
        CreateAnnouncementVal,
        CommentInfoVal,
        CreateCommentInstr,
        CreateCommentVal,
        TermbaseInfoVal,
        CreateTermbaseInstr,
        CreateTermbaseVal,
        UpdateTermbaseInfoInstr,
        TermInfoVal,
        CreateTermInstr,
        CreateTermVal,
        UpdateTermInfoInstr,
        MemberInfoVal,
        CreateMemberInstr,
        CreateMemberVal,
        JoinTeamInstr,
        UpdateMemberRolesInstr,
        MemberInvitationInfoVal,
        CreateMemberInvitationInstr,
        CreateMemberInvitationVal,
        UpdateMemberInvitationRolesInstr,
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
