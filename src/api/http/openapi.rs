//! OpenAPI documentation assembly.
//!
//! Handler-level `#[utoipa::path]` attributes specify full paths (including
//! the `/api/v1` prefix). Swagger UI and the OpenAPI JSON are mounted in debug
//! builds only, outside `/api/v1`.
use utoipa::OpenApi;

use crate::api::http::handler;
use crate::api::http::object::announcement::{
    AnnouncementInfoObj, CreateAnnouncementPayload, UpdateAnnouncementInfoBody,
};
use crate::api::http::object::assignment::{
    AssignmentInfoObj, JoinChapterAssignmentBody, UpdateAssignmentRolesBody,
};
use crate::api::http::object::assignment_invitation::{
    AssignmentInvitationInfoObj, CreateAssignmentInvitationPayload,
    JoinAssignmentInvitationBody,
};
use crate::api::http::object::auth::{LoginAuthPayload, RegisterAuthPayload};
use crate::api::http::object::chapter::{
    ChapterInfoObj, ChapterStageBody, ChapterStageOperBody,
    CreateChapterPayload, UpdateChapterInfoBody, UpdateChapterStageBody,
};
use crate::api::http::object::chapter_port::{
    ChapterTranslationFormatBody, ChapterTranslationImportModeBody,
    ChapterTranslationPortObj, ImportChapterTranslationPayload,
};
use crate::api::http::object::chapter_workflow_record::{
    ChapterWorkflowRecordEventObj, ChapterWorkflowRecordInfoObj,
    ChapterWorkflowRecordOriginObj, ChapterWorkflowRecordStageObj,
    ChapterWorkflowRecordStagePhaseObj,
    ChapterWorkflowRecordTranslationFormatObj,
};
use crate::api::http::object::comic::{
    AllocComicCoverPayload, ComicInfoObj, CreateComicPayload,
    MarkComicCoverUploadedBody, UpdateComicInfoBody,
};
use crate::api::http::object::comic_archive::ArchiveComicPayload;
use crate::api::http::object::comic_list::ListComicInfosPayload;
use crate::api::http::object::comment::{CommentInfoObj, CreateCommentPayload};
use crate::api::http::object::issue::{
    ImportChapterIssuesPayload, IssueBody, IssueInfoObj, IssueRectBody,
    PageIssuesBody,
};
use crate::api::http::object::issue_edit::{
    CreatedIssueIdPayload, SavePageIssuesPayload,
};
use crate::api::http::object::member::{
    CreateMemberPayload, JoinTeamBody, MemberInfoObj, UpdateMemberRolesBody,
};
use crate::api::http::object::member_invitation::{
    CreateMemberInvitationPayload, MemberInvitationInfoObj,
    UpdateMemberInvitationRolesBody,
};
use crate::api::http::object::page_artwork::{
    AllocChapterPageArtworksPayload, AllocPageArtworkImageBody,
    AllocatedPageArtworkPayload, MarkPageArtworkImageUploadedBody,
    PageArtworkImageBody, PageArtworkInfoObj,
};
use crate::api::http::object::page_source::{
    AllocChapterPageSourcesPayload, AllocPageSourceImageBody,
    AllocatedPageSourcePayload, MarkPageSourceImageUploadedBody,
    PageSourceImageBody, PageSourceInfoObj, PageSourceUnitDiffStatsPayload,
    PageSourceUnitFlaggedStatsPayload,
};
use crate::api::http::object::page_source_port::PageSourceTranslationPortObj;
use crate::api::http::object::system_mail::{
    MarkSystemMailReadBody, SystemMailInfoObj,
};
use crate::api::http::object::team::{
    AllocTeamAvatarPayload, CreateTeamBody, MarkTeamAvatarUploadedBody,
    TeamInfoObj, UpdateTeamInfoBody,
};
use crate::api::http::object::term::{
    CreateTermPayload, TermInfoObj, UpdateTermInfoBody,
};
use crate::api::http::object::termbase::{
    CreateTermbasePayload, TermbaseInfoObj, UpdateTermbaseInfoBody,
};
use crate::api::http::object::termbase_port::{
    AppendTermbasePayload, AppendTermbaseTermBody, ExportTermbasePayload,
    ImportTermBody, ImportTermbasePayload, TermbaseTermObj,
};
use crate::api::http::object::unit::{
    ListPageSourceUnitInfosPayload, TransformChapterUnitsBody, UnitCoordBody,
    UnitEditBody, UnitInfoObj, UnitRevisionBody, UnitTextTransformBody,
    UnitTransformBody, UnitTranslationBody,
};
use crate::api::http::object::unit_port::UnitTranslationPortObj;
use crate::api::http::object::upload::ImageUploadSlotObj;
use crate::api::http::object::user::{
    AllocUserAvatarPayload, MarkUserAvatarUploadedBody, UpdateUserInfoBody,
    UpdateUserPasswordBody, UserInfoObj,
};
use crate::api::http::object::workset::{
    CreateWorksetPayload, UpdateWorksetInfoBody, WorksetInfoObj,
};
use crate::api::http::result::HttpError;
use crate::value::announcement::AnnouncementInclOpt;
use crate::value::assignment::AssignmentInclOpt;
use crate::value::chapter::ChapterInclOpt;
use crate::value::comic::{ComicInclOpt, ComicStatus, ComicWithOpt};
use crate::value::comment::CommentInclOpt;
use crate::value::member::MemberInclOpt;
use crate::value::member_invitation::MemberInvitationInclOpt;
use crate::value::role::RoleField;
use crate::value::unit::UnitTextPart;

/// Top-level `OpenAPI` document for the `PopRaKo` HTTP API.
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
        handler::user::alloc_avatar,
        handler::user::mark_avatar_uploaded,
        handler::team::create,
        handler::team::list_infos,
        handler::team::get_info,
        handler::team::update_info,
        handler::team::alloc_avatar,
        handler::team::mark_avatar_uploaded,
        handler::team::list_online_user_ids,
        handler::team::mark_self_online,
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
        handler::comic::alloc_cover,
        handler::comic::mark_cover_uploaded,
        handler::comic::archive,
        handler::comic::export_archives,
        handler::comic::delete,
        handler::chapter::create,
        handler::chapter::list_infos,
        handler::chapter::get_pinned,
        handler::chapter::get_info,
        handler::chapter::list_workflow_record_infos,
        handler::chapter::update_info,
        handler::chapter::mark_pinned,
        handler::chapter::advance_stage,
        handler::chapter::delete,
        handler::chapter_port::alloc_artwork,
        handler::chapter_port::mark_artwork_uploaded,
        handler::chapter_port::export_artwork,
        handler::chapter_port::import_translation,
        handler::chapter_port::export_translation,
        handler::chapter_port::export_translation_download,
        handler::page_artwork::list_infos,
        handler::page_artwork::alloc_chapter_page_artworks,
        handler::page_artwork::alloc_image,
        handler::page_artwork::mark_image_uploaded,
        handler::issue::import_issue,
        handler::issue::list_infos,
        handler::issue_edit::get_info,
        handler::issue_edit::save_infos,
        handler::issue_edit::list_variant_infos,
        handler::issue_edit::rename_variant,
        handler::page_source::list_infos,
        handler::page_source::list_unit_diff_stats,
        handler::page_source::list_unit_flagged_stats,
        handler::page_source::get_info,
        handler::page_source::delete,
        handler::page_source::alloc_chapter_page_sources,
        handler::page_source::alloc_image,
        handler::page_source::mark_image_uploaded,
        handler::unit::list_infos,
        handler::unit::search_infos,
        handler::unit::save_infos,
        handler::unit::transform,
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
        handler::announcement::update_info,
        handler::announcement::delete,
        handler::comment::create,
        handler::comment::list_infos,
        handler::termbase::create,
        handler::termbase::list_team_infos,
        handler::termbase::list_comic_infos,
        handler::termbase::get_info,
        handler::termbase::update_info,
        handler::termbase::delete,
        handler::termbase_port::append,
        handler::termbase_port::export,
        handler::termbase_port::export_download,
        handler::termbase_port::import_team,
        handler::termbase_port::import_comic,
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
        ImportChapterIssuesPayload, PageIssuesBody, IssueBody, IssueRectBody,
        SavePageIssuesPayload,
        CreatedIssueIdPayload, IssueInfoObj,
        AllocChapterPageArtworksPayload, AllocPageArtworkImageBody,
        MarkPageArtworkImageUploadedBody, PageArtworkImageBody,
        AllocatedPageArtworkPayload,
            PageArtworkInfoObj,
        HttpError,
        RegisterAuthPayload,
        LoginAuthPayload,
        UserInfoObj,
        UpdateUserInfoBody,
        UpdateUserPasswordBody,
        AllocUserAvatarPayload,
        MarkUserAvatarUploadedBody,
        TeamInfoObj,
        CreateTeamBody,
        UpdateTeamInfoBody,
        AllocTeamAvatarPayload,
        MarkTeamAvatarUploadedBody,
        WorksetInfoObj,
        CreateWorksetPayload,
        UpdateWorksetInfoBody,
        ComicInfoObj,
        ComicStatus,
        ListComicInfosPayload,
        CreateComicPayload,
        UpdateComicInfoBody,
        AllocComicCoverPayload,
        MarkComicCoverUploadedBody,
        ArchiveComicPayload,
        ChapterInfoObj,
        ChapterWorkflowRecordEventObj,
        ChapterWorkflowRecordInfoObj,
        ChapterWorkflowRecordOriginObj,
        ChapterWorkflowRecordStagePhaseObj,
        ChapterWorkflowRecordStageObj,
        ChapterWorkflowRecordTranslationFormatObj,
        CreateChapterPayload,
        UpdateChapterInfoBody,
        ChapterStageBody,
        ChapterStageOperBody,
        UpdateChapterStageBody,
        ChapterTranslationPortObj,
        PageSourceTranslationPortObj,
        UnitTranslationPortObj,
        ChapterTranslationFormatBody,
        ChapterTranslationImportModeBody,
        ImportChapterTranslationPayload,
        PageSourceInfoObj,
        PageSourceImageBody,
        ImageUploadSlotObj,
        AllocChapterPageSourcesPayload,
        AllocPageSourceImageBody,
        AllocatedPageSourcePayload,
        MarkPageSourceImageUploadedBody,
        UnitInfoObj,
        ListPageSourceUnitInfosPayload,
        PageSourceUnitDiffStatsPayload, PageSourceUnitFlaggedStatsPayload,
        UnitEditBody,
        UnitCoordBody,
        UnitTranslationBody,
        UnitRevisionBody,
        UnitTextPart,
        TransformChapterUnitsBody,
        UnitTransformBody,
        UnitTextTransformBody,
        AssignmentInfoObj,
        JoinChapterAssignmentBody,
        UpdateAssignmentRolesBody,
        AssignmentInvitationInfoObj,
        CreateAssignmentInvitationPayload,
        JoinAssignmentInvitationBody,
        SystemMailInfoObj,
        MarkSystemMailReadBody,
        AnnouncementInfoObj,
        CreateAnnouncementPayload,
        UpdateAnnouncementInfoBody,
        CommentInfoObj,
        CreateCommentPayload,
        TermbaseInfoObj,
        CreateTermbasePayload,
        UpdateTermbaseInfoBody,
        AppendTermbaseTermBody,
        AppendTermbasePayload,
        ImportTermBody,
        ImportTermbasePayload,
        TermbaseTermObj,
        ExportTermbasePayload,
        TermInfoObj,
        CreateTermPayload,
        UpdateTermInfoBody,
        MemberInfoObj,
        CreateMemberPayload,
        JoinTeamBody,
        UpdateMemberRolesBody,
        MemberInvitationInfoObj,
        CreateMemberInvitationPayload,
        UpdateMemberInvitationRolesBody,
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
        (name = "chapter-port",
            description = "Chapter translation and artwork import/export \
            endpoints"),
        (name = "page-artworks",
            description = "Composite artwork page endpoints"),
        (name = "issues", description = "Composite artwork review endpoints"),
        (name = "page-sources",
            description = "Source page management endpoints"),
        (name = "units", description = "Source page unit endpoints"),
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
        (name = "termbase-port",
            description = "Terminology-base import/export endpoints"),
    )
)]
pub struct ApiDoc;
