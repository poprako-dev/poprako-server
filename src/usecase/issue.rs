//! Import and read the Chapter's single current review.

#[cfg(test)]
mod tests;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use crate::complex::{chapter as chapter_complex, issue as issue_complex};
use crate::data::instr::issue::ImportChapterIssuesInstr;
use crate::data::val::issue::ImportChapterIssuesVal;
use crate::data::view::issue::IssueInfoView;
use crate::model::shared::user::UserToken;
use crate::model::write::issue::{ChapterIssuesRepl, IssueEntry};
use crate::part::nucl::ReptRead;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::issue::IssueRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::GetChapterInfoExcluded;
use crate::part::repo::oper::comic::TouchComicLastActive;
use crate::part::repo::oper::issue::{ListIssueInfos, ReplaceChapterIssues};
use crate::part::repo::oper::page::{GetPageInfo, ListPageInfosExcluded};
use crate::part::repo::page::PageRepo;
use crate::part::repo::team::TeamRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::page::list as page_list_usecase;
use crate::util::next_snowflake_id;

/// Atomically replaces all issues without changing workflow or Unit counters.
#[instrument(level = "info", skip(nucl, repo, token, instr), fields(actor_user_id = %token.user_id))]
pub async fn import<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    chapter_id: String,
    instr: ImportChapterIssuesInstr,
) -> BaseRest<ImportChapterIssuesVal>
where
    C: Context + Send,
    C::Level: AtLeast<ReptRead>,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    R: ChapterRepo<C>
        + PageRepo<C>
        + AssignmentRepo<C>
        + IssueRepo<C>
        + ComicRepo<C>
        + Send
        + Sync,
{
    let imported = nucl
        .coord(async move |context| {
            //
            let chapter_info = GetChapterInfoExcluded {
                id: &chapter_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            let page_infos = ListPageInfosExcluded {
                chapter_id: &chapter_info.id,
            }
            .step_on(repo, context)
            .await?;

            let assignment_info = FindAssignmentInfo::ChapterUser {
                chapter_id: &chapter_info.id,
                user_id: &token.user_id,
            }
            .step_on(repo, context)
            .await?
            .ok_or_else(|| {
                //
                issue_complex::error(
                    ExpectedVariant::Perm,
                    "error-issue-reviewer-required",
                )
            })?;

            issue_complex::ensure_user_can_import(&assignment_info)?;

            chapter_complex::ensure_chapter_writable(&chapter_info)?;

            issue_complex::ensure_import(&instr, page_infos.len())?;

            let mut entries = Vec::new();

            for (page_info, page_instr) in page_infos.iter().zip(instr.pages) {
                //
                for (index, issue_instr) in
                    page_instr.issues.into_iter().enumerate()
                {
                    let issue_entry = IssueEntry {
                        id: next_snowflake_id(),
                        page_id: page_info.id.clone(),
                        index,
                        variant: issue_instr.variant,
                        layer_path: issue_instr.layer_path,
                        rect: issue_instr.rect.map(Into::into),
                        note: issue_instr.note,
                    };

                    entries.push(issue_entry);
                }
            }

            let chapter_issues_repl = ChapterIssuesRepl {
                chapter_id: &chapter_info.id,
                entries: &entries,
            };

            ReplaceChapterIssues {
                repl: &chapter_issues_repl,
            }
            .step_on(repo, context)
            .await?;

            TouchComicLastActive {
                id: &chapter_info.comic_id,
            }
            .step_on(repo, context)
            .await?;

            accept(ImportChapterIssuesVal {
                imported_page_count: page_infos.len(),
                imported_issue_count: entries.len(),
            })
        })
        .await?;

    accept(imported)
}

/// Reads a Page's ordered issues using the same access rules as Page information.
#[instrument(level = "info", skip(repo, token), fields(actor_user_id = %token.user_id))]
pub async fn list_infos<C, R>(
    (repo,): (&R,),
    token: UserToken,
    page_id: String,
) -> BaseRest<Vec<IssueInfoView>>
where
    C: Context,
    R: PageRepo<C>
        + IssueRepo<C>
        + TeamRepo<C>
        + MemberRepo<C>
        + AssignmentRepo<C>
        + Sync,
{
    let page_info = GetPageInfo { id: &page_id }.run_on(repo).await?;

    page_list_usecase::ensure_user_can_list_infos::<C, R>(
        repo,
        &token,
        &page_info.chapter_id,
    )
    .await?;

    let issue_infos = ListIssueInfos {
        page_id: &page_info.id,
    }
    .run_on(repo)
    .await?;

    accept(issue_infos.into_iter().map(Into::into).collect())
}
