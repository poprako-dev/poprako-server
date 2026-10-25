//! source page read orchestration.

use poprako_orchestra::{Context, OperRun as _};
use tracing::instrument;

use poprako_obj_dept::ObjDeptView;
use poprako_util::i18n::trl;

use crate::complex::page_source::perm as page_source_perm_complex;
use crate::data::instr::page_source::{
    ListPageSourceInfosInstr, ListPageSourceUnitDiffStatsInstr,
    ListPageSourceUnitFlaggedStatsInstr,
};
use crate::data::val::page_source::{
    PageSourceUnitDiffStatsVal, PageSourceUnitFlaggedStatsVal,
};
use crate::data::view::page_source::PageSourceInfoView;
use crate::model::shared::user::UserToken;
use crate::part::obj_dept::PageSourceImage;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::page_source::{
    GetPageSourceInfo, ListPageSourceInfos, ListPageSourceUnitDiffStats,
    ListPageSourceUnitFlaggedStats,
};
use crate::part::repo::oper::team::ResolveTeamId;
use crate::part::repo::page_source::PageSourceRepo;
use crate::part::repo::team::TeamRepo;
use crate::result::{BaseRest, ExpectedVariant};
use crate::usecase::page_source::view::{
    page_source_info_view, page_source_info_views,
};

/// Lists pages under one chapter.
#[instrument(
    level = "info",
    skip(repo, obj_dept, token),
    fields(actor_user_id = %token.user_id),
)]
pub async fn list_infos<C, R, O>(
    (repo, obj_dept): (&R, &O),
    token: UserToken,
    instr: ListPageSourceInfosInstr,
) -> BaseRest<Vec<PageSourceInfoView>>
where
    C: Context,
    R: PageSourceRepo<C>
        + TeamRepo<C>
        + MemberRepo<C>
        + AssignmentRepo<C>
        + Sync,
    O: ObjDeptView<PageSourceImage, C> + Sync,
{
    ensure_user_can_list_infos::<C, R>(repo, &token, &instr.chapter_id).await?;

    let page_source_infos = ListPageSourceInfos {
        chapter_id: &instr.chapter_id,
    }
    .run_on(repo)
    .await?;

    page_source_info_views((repo, obj_dept), page_source_infos).await
}

/// Lists Unit text statistics for source pages containing visible revision differences.
#[instrument(
    level = "info",
    skip(repo, token),
    fields(chapter_id = %instr.chapter_id),
)]
pub async fn list_unit_diff_stats<C, R>(
    (repo,): (&R,),
    token: UserToken,
    instr: ListPageSourceUnitDiffStatsInstr,
) -> BaseRest<Vec<PageSourceUnitDiffStatsVal>>
where
    C: Context,
    R: PageSourceRepo<C>
        + TeamRepo<C>
        + MemberRepo<C>
        + AssignmentRepo<C>
        + Sync,
{
    ensure_user_can_list_infos::<C, R>(repo, &token, &instr.chapter_id).await?;

    let page_source_unit_diff_stats = ListPageSourceUnitDiffStats {
        chapter_id: &instr.chapter_id,
    }
    .run_on(repo)
    .await?;

    accept!(
        page_source_unit_diff_stats
            .into_iter()
            .map(Into::into)
            .collect()
    )
}

/// Lists flagged Unit statistics for source pages containing visible flagged Units.
#[instrument(
    level = "info",
    skip(repo, token),
    fields(chapter_id = %instr.chapter_id),
)]
pub async fn list_unit_flagged_stats<C, R>(
    (repo,): (&R,),
    token: UserToken,
    instr: ListPageSourceUnitFlaggedStatsInstr,
) -> BaseRest<Vec<PageSourceUnitFlaggedStatsVal>>
where
    C: Context,
    R: PageSourceRepo<C>
        + TeamRepo<C>
        + MemberRepo<C>
        + AssignmentRepo<C>
        + Sync,
{
    ensure_user_can_list_infos(repo, &token, &instr.chapter_id).await?;

    let page_source_unit_flagged_stats = ListPageSourceUnitFlaggedStats {
        chapter_id: &instr.chapter_id,
    }
    .run_on(repo)
    .await?;

    accept!(
        page_source_unit_flagged_stats
            .into_iter()
            .map(Into::into)
            .collect(),
    )
}

/// Fetches one page by ID.
#[instrument(
    level = "info",
    skip(repo, obj_dept, token),
    fields(actor_user_id = %token.user_id),
)]
pub async fn get_info<C, R, O>(
    (repo, obj_dept): (&R, &O),
    token: UserToken,
    id: String,
) -> BaseRest<PageSourceInfoView>
where
    C: Context,
    R: PageSourceRepo<C>
        + TeamRepo<C>
        + MemberRepo<C>
        + AssignmentRepo<C>
        + Sync,
    O: ObjDeptView<PageSourceImage, C> + Sync,
{
    let page_source_info = GetPageSourceInfo { id: &id }.run_on(repo).await?;

    ensure_user_can_list_infos::<C, R>(
        repo,
        &token,
        &page_source_info.chapter_id,
    )
    .await?;

    page_source_info_view((repo, obj_dept), page_source_info).await
}

/// Loads concrete membership or assignment evidence for source page read access.
pub async fn ensure_user_can_list_infos<C, R>(
    repo: &R,
    token: &UserToken,
    chapter_id: &str,
) -> BaseRest<()>
where
    C: Context,
    R: TeamRepo<C> + MemberRepo<C> + AssignmentRepo<C> + Sync,
{
    let team_id = ResolveTeamId::Chapter { id: chapter_id }
        .run_on(repo)
        .await?;

    let member_info = FindMemberInfo {
        user_id: &token.user_id,
        team_id: &team_id,
    }
    .run_on(repo)
    .await?;

    if let Some(member_info) = member_info {
        //
        return page_source_perm_complex::ensure_user_can_list_infos(
            &page_source_perm_complex::PageSourceListAccess::Member {
                member_info: &member_info,
            },
        );
    }

    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id,
        user_id: &token.user_id,
    }
    .run_on(repo)
    .await?;

    let Some(assignment_info) = assignment_info else {
        //
        return Err(expected_error! {
            ExpectedVariant::Perm,
            message: trl("error-forbidden");
            chapter_id = %chapter_id,
            user_id = %token.user_id,
            "page list permission denied",
        });
    };

    page_source_perm_complex::ensure_user_can_list_infos(
        &page_source_perm_complex::PageSourceListAccess::Assignee {
            assignment_info: &assignment_info,
        },
    )
}
