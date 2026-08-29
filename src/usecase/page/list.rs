//! Page read orchestration.

use poprako_orchestra::{Context, OperRun as _, Run};
use tracing::instrument;

use poprako_obj_dept::oper::{GenObjUrl, GetObjMeta};
use poprako_obj_dept::rest::ObjDeptError;
use poprako_util::i18n::trl;

use crate::complex::page::{PageListAccess, PagePermComplex};
use crate::data::instr::page::ListPageInfosInstr;
use crate::data::view::page::PageInfoView;
use crate::model::shared::user::UserToken;
use crate::part::obj_dept::PageImage;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::page::{GetPageInfo, ListPageInfos};
use crate::part::repo::oper::team::ResolveTeamId;
use crate::part::repo::page::PageRepo;
use crate::part::repo::team::TeamRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant};
use crate::usecase::internal::util::collect_bounded;
use crate::usecase::view::page_info_view;

/// Lists pages under one chapter.
#[instrument(level = "info", skip(repo, obj_dept))]
pub async fn list_infos<C, R, O>(
    (repo, obj_dept): (&R, &O),
    token: UserToken,
    instr: ListPageInfosInstr,
) -> BaseRest<Vec<PageInfoView>>
where
    C: Context,
    R: PageRepo<C> + TeamRepo<C> + MemberRepo<C> + AssignmentRepo<C> + Sync,
    O: for<'a> Run<GetObjMeta<'a, PageImage>, Error = ObjDeptError>
        + for<'a> Run<GenObjUrl<'a, PageImage>, Error = ObjDeptError>
        + Sync,
{
    ensure_user_can_list_infos::<C, R>(repo, &token, &instr.chapter_id).await?;

    let page_infos = ListPageInfos {
        chapter_id: &instr.chapter_id,
    }
    .run_on(repo)
    .await?;

    collect_bounded(
        page_infos
            .into_iter()
            .map(|page_info| page_info_view(obj_dept, page_info)),
    )
    .await
}

/// Fetches one page by ID.
#[instrument(level = "info", skip(repo, obj_dept))]
pub async fn get_info<C, R, O>(
    (repo, obj_dept): (&R, &O),
    token: UserToken,
    id: String,
) -> BaseRest<PageInfoView>
where
    C: Context,
    R: PageRepo<C> + TeamRepo<C> + MemberRepo<C> + AssignmentRepo<C> + Sync,
    O: for<'a> Run<GetObjMeta<'a, PageImage>, Error = ObjDeptError>
        + for<'a> Run<GenObjUrl<'a, PageImage>, Error = ObjDeptError>
        + Sync,
{
    let page_info = GetPageInfo { id: &id }.run_on(repo).await?;

    ensure_user_can_list_infos::<C, R>(repo, &token, &page_info.chapter_id)
        .await?;

    page_info_view(obj_dept, page_info).await
}

// Load concrete membership or assignment evidence for page-list access.
async fn ensure_user_can_list_infos<C, R>(
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

    let member_info = FindMemberInfo::UserTeam {
        user_id: &token.user_id,
        team_id: &team_id,
    }
    .run_on(repo)
    .await?;

    if let Some(member_info) = member_info {
        //
        return PagePermComplex::ensure_user_can_list_infos(
            &PageListAccess::Member {
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
        let err_message = trl("error-forbidden");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            chapter_id = %chapter_id,
            user_id = %token.user_id,
            "expected error: page list permission denied",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    };

    PagePermComplex::ensure_user_can_list_infos(&PageListAccess::Assignee {
        assignment_info: &assignment_info,
    })
}
